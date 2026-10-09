//! All persistence, written once against [`Db`]. SQLite and D1 only provide `Db`.

use jiff::Timestamp;
use jiff::civil::Date;

use crate::db::{Db, Stmt, Value};
use crate::model::{
    NewFeedback, NewPick, PickId, Restaurant, RestaurantId, RestaurantInput, Session, SessionId, SessionStatus,
    SessionView, Settings, User, UserId, UserPrefs,
};
use crate::pipeline::PickContext;
use crate::rows::{self, IdRow};
use crate::{Error, Result, sql};

pub struct Store<D> {
    db: D,
}

fn secs(t: Timestamp) -> Value {
    Value::Int(t.as_second())
}

fn day(d: Date) -> Value {
    Value::Text(d.to_string())
}

fn first_id(ids: Vec<IdRow>) -> Result<i64> {
    ids.first()
        .map(|r| r.id)
        .ok_or_else(|| Error::Storage("insert returned no id".into()))
}

impl<D: Db> Store<D> {
    pub fn new(db: D) -> Self {
        Store { db }
    }

    pub fn db(&self) -> &D {
        &self.db
    }

    pub fn into_db(self) -> D {
        self.db
    }

    pub async fn settings(&self) -> Result<Settings> {
        rows::settings(&self.db.all::<rows::SettingRow>(sql::SETTINGS, vec![]).await?)
    }

    pub async fn upsert_user(&self, user: &User, now: Timestamp) -> Result<()> {
        let params = vec![user.id.as_str().into(), user.display_name.as_str().into(), secs(now)];
        self.db.batch(vec![Stmt::new(sql::UPSERT_USER, params)]).await
    }

    /// Like `upsert_user`, but keeps an existing display name.
    pub async fn ensure_user(&self, user: &User, now: Timestamp) -> Result<()> {
        let params = vec![user.id.as_str().into(), user.display_name.as_str().into(), secs(now)];
        self.db.batch(vec![Stmt::new(sql::ENSURE_USER, params)]).await
    }

    pub async fn prefs(&self, user: &UserId) -> Result<UserPrefs> {
        let p = || vec![Value::from(user.as_str())];
        let found = rows::users_with_prefs(
            self.db.all(sql::USER, p()).await?,
            self.db.all(sql::USER_TAGS, p()).await?,
            self.db.all(sql::USER_CUISINES, p()).await?,
            self.db.all(sql::USER_STANCES, p()).await?,
            self.db.all(sql::USER_RATINGS, p()).await?,
        )?;
        Ok(found.into_iter().next().map(|(_, prefs)| prefs).unwrap_or_default())
    }

    /// Replaces a user's preferences atomically (ratings are feedback and are not touched). The user must already exist.
    pub async fn replace_prefs(&self, user: &UserId, prefs: &UserPrefs) -> Result<()> {
        let id = || Value::from(user.as_str());
        let mut stmts = vec![
            Stmt::new(
                sql::UPDATE_USER_LIMITS,
                vec![id(), prefs.max_walk_min.into(), prefs.price_pref.into()],
            ),
            Stmt::new(sql::CLEAR_USER_TAGS, vec![id()]),
            Stmt::new(sql::CLEAR_USER_CUISINES, vec![id()]),
            Stmt::new(sql::CLEAR_USER_STANCES, vec![id()]),
        ];
        stmts.extend(
            prefs
                .constraints
                .iter()
                .map(|t| Stmt::new(sql::INSERT_USER_TAG, vec![id(), t.as_str().into()])),
        );
        stmts.extend(prefs.cuisine_weights.iter().map(|(cuisine, weight)| {
            Stmt::new(
                sql::INSERT_USER_CUISINE,
                vec![id(), cuisine.as_str().into(), Value::Int(i64::from(*weight))],
            )
        }));
        stmts.extend(
            prefs
                .favorites
                .iter()
                .map(|r| Stmt::new(sql::UPSERT_USER_STANCE, vec![id(), Value::Int(*r), "favorite".into()])),
        );
        self.db.batch(stmts).await
    }

    pub async fn get_or_create_session(
        &self,
        channel: &str,
        date: Date,
        created_by: &UserId,
        now: Timestamp,
    ) -> Result<Session> {
        let insert = vec![channel.into(), day(date), created_by.as_str().into(), secs(now)];
        self.db
            .batch(vec![Stmt::new(sql::INSERT_SESSION_IF_MISSING, insert)])
            .await?;
        let row = self
            .db
            .all::<rows::SessionRow>(sql::SESSION_BY_CHANNEL_DATE, vec![channel.into(), day(date)])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::Storage("session vanished after insert".into()))?;
        rows::session(row)
    }

    pub async fn session(&self, id: SessionId) -> Result<Session> {
        let row = self
            .db
            .all::<rows::SessionRow>(sql::SESSION_BY_ID, vec![Value::Int(id)])
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::NotFound(format!("session {id}")))?;
        rows::session(row)
    }

    pub async fn set_message_ts(&self, id: SessionId, ts: &str) -> Result<()> {
        self.db
            .batch(vec![Stmt::new(sql::SET_MESSAGE_TS, vec![Value::Int(id), ts.into()])])
            .await
    }

    pub async fn set_participation(&self, id: SessionId, user: &UserId, joined: bool, now: Timestamp) -> Result<()> {
        let stmt = if joined {
            Stmt::new(sql::JOIN, vec![Value::Int(id), user.as_str().into(), secs(now)])
        } else {
            Stmt::new(sql::LEAVE, vec![Value::Int(id), user.as_str().into()])
        };
        self.db.batch(vec![stmt]).await
    }

    pub async fn restaurants(&self, include_inactive: bool) -> Result<Vec<Restaurant>> {
        rows::restaurants(
            self.db.all(sql::RESTAURANTS, vec![include_inactive.into()]).await?,
            self.db.all(sql::DIET_SUPPORT, vec![]).await?,
        )
    }

    pub async fn restaurant(&self, id: RestaurantId) -> Result<Restaurant> {
        rows::restaurants(
            self.db.all(sql::RESTAURANT, vec![Value::Int(id)]).await?,
            self.db.all(sql::DIET_SUPPORT_FOR, vec![Value::Int(id)]).await?,
        )?
        .into_iter()
        .next()
        .ok_or_else(|| Error::NotFound(format!("restaurant {id}")))
    }

    pub async fn save_restaurant(&self, input: &RestaurantInput, now: Timestamp) -> Result<RestaurantId> {
        let params = vec![
            input.name.trim().into(),
            input.address.clone().into(),
            input.walk_min.into(),
            input.cuisine.trim().to_lowercase().into(),
            input.price_level.into(),
            input.opening_hours.clone().into(),
            input.menu_url.clone().into(),
            secs(now),
        ];
        first_id(self.db.all(sql::INSERT_RESTAURANT, params).await?)
    }

    pub async fn session_view(&self, id: SessionId) -> Result<SessionView> {
        let session = self.session(id).await?;
        let participants = self
            .db
            .all::<rows::UserRow>(sql::PARTICIPANTS, vec![Value::Int(id)])
            .await?
            .into_iter()
            .map(|r| User {
                id: r.id,
                display_name: r.display_name,
            })
            .collect();
        let latest = self
            .db
            .all::<rows::PickRow>(sql::LATEST_PICK, vec![Value::Int(id)])
            .await?;
        let latest_pick = match latest.into_iter().next() {
            Some(row) => {
                let restaurant = self.restaurant(row.restaurant_id).await?;
                Some(rows::shown_pick(row, restaurant)?)
            }
            None => None,
        };
        Ok(SessionView {
            session,
            participants,
            latest_pick,
        })
    }

    pub async fn pick_context(&self, id: SessionId, today: Date) -> Result<PickContext> {
        let s = || vec![Value::Int(id)];
        Ok(PickContext {
            settings: self.settings().await?,
            today,
            participants: rows::users_with_prefs(
                self.db.all(sql::PARTICIPANTS, s()).await?,
                self.db.all(sql::PARTICIPANT_TAGS, s()).await?,
                self.db.all(sql::PARTICIPANT_CUISINES, s()).await?,
                self.db.all(sql::PARTICIPANT_STANCES, s()).await?,
                self.db.all(sql::PARTICIPANT_RATINGS, s()).await?,
            )?,
            restaurants: self.restaurants(false).await?,
            offers_today: rows::offers(self.db.all(sql::OFFERS_FOR_DATE, vec![day(today)]).await?)?,
            last_accepted: rows::last_accepted(self.db.all(sql::LAST_ACCEPTED, vec![day(today)]).await?)?,
            already_shown: self
                .db
                .all::<IdRow>(sql::SHOWN_IN_SESSION, s())
                .await?
                .into_iter()
                .map(|r| r.id)
                .collect(),
        })
    }

    /// Records a roll or manual pick and re-opens the session: a new pick un-decides it and
    /// clears any earlier acceptance, so history only counts places the group actually chose.
    pub async fn insert_pick(&self, pick: &NewPick) -> Result<PickId> {
        let explanation = serde_json::to_string(&pick.explanation).map_err(|e| Error::Storage(e.to_string()))?;
        let params = vec![
            Value::Int(pick.session_id),
            Value::Int(pick.restaurant_id),
            pick.kind.as_str().into(),
            explanation.into(),
            secs(pick.created_at),
        ];
        let id = first_id(self.db.all(sql::INSERT_PICK, params).await?)?;
        let reopen = vec![Value::Int(pick.session_id), SessionStatus::Open.as_str().into()];
        self.db
            .batch(vec![
                Stmt::new(sql::CLEAR_ACCEPTED, vec![Value::Int(pick.session_id)]),
                Stmt::new(sql::SET_SESSION_STATUS, reopen),
            ])
            .await?;
        Ok(id)
    }

    /// Accepts `pick`, which must still be the session's latest pick (a re-roll may have replaced it).
    pub async fn accept_pick(&self, session: SessionId, pick: PickId) -> Result<()> {
        let latest = self
            .db
            .all::<rows::PickRow>(sql::LATEST_PICK, vec![Value::Int(session)])
            .await?;
        if latest.first().map(|row| row.id) != Some(pick) {
            let exists = !self
                .db
                .all::<IdRow>(sql::PICK_IN_SESSION, vec![Value::Int(session), Value::Int(pick)])
                .await?
                .is_empty();
            return Err(if exists {
                Error::Invalid("a newer pick replaced this one — check the board".into())
            } else {
                Error::NotFound(format!("pick {pick} in session {session}"))
            });
        }
        self.db
            .batch(vec![
                Stmt::new(sql::ACCEPT_PICK, vec![Value::Int(session), Value::Int(pick)]),
                Stmt::new(
                    sql::SET_SESSION_STATUS,
                    vec![Value::Int(session), SessionStatus::Decided.as_str().into()],
                ),
            ])
            .await
    }

    pub async fn pick_restaurant(&self, session: SessionId, pick: PickId) -> Result<RestaurantId> {
        let rows = self
            .db
            .all::<IdRow>(sql::PICK_RESTAURANT, vec![Value::Int(session), Value::Int(pick)])
            .await?;
        rows.first()
            .map(|r| r.id)
            .ok_or_else(|| Error::NotFound(format!("pick {pick} in session {session}")))
    }

    /// Records (or changes) a person's rating of one place in one session.
    pub async fn add_feedback(&self, f: &NewFeedback) -> Result<()> {
        let params = vec![
            f.user_id.as_str().into(),
            Value::Int(f.restaurant_id),
            Value::Int(f.session_id),
            f.rating.as_str().into(),
            secs(f.created_at),
        ];
        self.db.batch(vec![Stmt::new(sql::UPSERT_FEEDBACK, params)]).await
    }
}
