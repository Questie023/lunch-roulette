//! Business flows, written once. The worker runs them over D1, the TUI (M2) over SQLite.

use jiff::civil::Date;
use rand::Rng;

use crate::clock::Clock;
use crate::db::Db;
use crate::geo::walk_minutes;
use crate::model::{
    Explanation, NewFeedback, NewPick, PickId, PickKind, PickOptions, PickOutcome, PickResponse, Rating, Reason,
    Restaurant, RestaurantId, RestaurantInput, SessionId, SessionView, User, UserId,
};
use crate::pipeline::run_pick;
use crate::rules::opening_hours::parse_opening_hours;
use crate::store::Store;
use crate::{Error, Result};

pub struct LunchService<D, C, G> {
    store: Store<D>,
    clock: C,
    rng: G,
}

impl<D: Db, C: Clock, G: Rng> LunchService<D, C, G> {
    pub fn new(store: Store<D>, clock: C, rng: G) -> Self {
        LunchService { store, clock, rng }
    }

    pub fn store(&self) -> &Store<D> {
        &self.store
    }

    pub fn into_store(self) -> Store<D> {
        self.store
    }

    /// Today's date in the office time zone.
    async fn today(&self) -> Result<Date> {
        let tz = self.store.settings().await?.time_zone()?;
        Ok(self.clock.now().to_zoned(tz).date())
    }

    /// Opens (or reuses) today's session in `channel` and auto-joins `actor`.
    pub async fn open_session(&mut self, channel: &str, actor: &User) -> Result<SessionView> {
        let now = self.clock.now();
        let today = self.today().await?;
        self.store.upsert_user(actor, now).await?;
        let session = self.store.get_or_create_session(channel, today, &actor.id, now).await?;
        self.store.set_participation(session.id, &actor.id, true, now).await?;
        self.store.session_view(session.id).await
    }

    pub async fn join(&mut self, session: SessionId, actor: &User) -> Result<SessionView> {
        let now = self.clock.now();
        self.store.session(session).await?;
        self.store.upsert_user(actor, now).await?;
        self.store.set_participation(session, &actor.id, true, now).await?;
        self.store.session_view(session).await
    }

    pub async fn leave(&mut self, session: SessionId, actor: &UserId) -> Result<SessionView> {
        self.store
            .set_participation(session, actor, false, self.clock.now())
            .await?;
        self.store.session_view(session).await
    }

    pub async fn view(&self, session: SessionId) -> Result<SessionView> {
        self.store.session_view(session).await
    }

    pub async fn set_message_ts(&mut self, session: SessionId, ts: &str) -> Result<()> {
        self.store.set_message_ts(session, ts).await
    }

    /// Rolls the roulette. Restaurants already shown in this session are skipped.
    pub async fn pick(&mut self, session: SessionId, opts: PickOptions) -> Result<PickResponse> {
        let today = self.today().await?;
        let ctx = self.store.pick_context(session, today).await?;
        if ctx.participants.is_empty() {
            return Err(Error::NoParticipants);
        }
        match run_pick(&ctx, opts, &mut self.rng) {
            PickOutcome::Picked {
                restaurant_id,
                explanation,
            } => {
                let pick = NewPick {
                    session_id: session,
                    restaurant_id,
                    kind: PickKind::Roll,
                    explanation,
                    created_at: self.clock.now(),
                };
                self.store.insert_pick(&pick).await?;
                Ok(PickResponse::Picked(self.store.session_view(session).await?))
            }
            PickOutcome::Empty { stage } => Ok(PickResponse::Empty {
                stage,
                view: self.store.session_view(session).await?,
            }),
        }
    }

    pub async fn manual_pick(&mut self, session: SessionId, restaurant: RestaurantId) -> Result<SessionView> {
        let chosen = self.store.restaurant(restaurant).await?;
        let settings = self.store.settings().await?;
        let explanation = Explanation {
            walk_min: walk_minutes(&chosen, &settings),
            reasons: vec![Reason::ManualChoice],
            warnings: vec![],
        };
        let pick = NewPick {
            session_id: session,
            restaurant_id: restaurant,
            kind: PickKind::Manual,
            explanation,
            created_at: self.clock.now(),
        };
        self.store.insert_pick(&pick).await?;
        self.store.session_view(session).await
    }

    pub async fn accept(&mut self, session: SessionId, pick: PickId) -> Result<SessionView> {
        self.store.accept_pick(session, pick).await?;
        self.store.session_view(session).await
    }

    /// Rates the pick the user was shown (`pick`), not whatever is latest by the time the click
    /// arrives. Anyone may rate, joined or not; an existing display name is never overwritten.
    pub async fn feedback(
        &mut self,
        session: SessionId,
        pick: PickId,
        actor: &User,
        rating: Rating,
    ) -> Result<RestaurantId> {
        let restaurant_id = self.store.pick_restaurant(session, pick).await?;
        let now = self.clock.now();
        self.store.ensure_user(actor, now).await?;
        let feedback = NewFeedback {
            session_id: session,
            user_id: actor.id.clone(),
            restaurant_id,
            rating,
            created_at: now,
        };
        self.store.add_feedback(&feedback).await?;
        Ok(restaurant_id)
    }

    pub async fn add_restaurant(&mut self, input: RestaurantInput) -> Result<RestaurantId> {
        input.validate()?;
        if let Some(hours) = &input.opening_hours {
            parse_opening_hours(hours).map_err(|e| Error::Invalid(e.to_string()))?;
        }
        let name = input.name.trim();
        if self
            .store
            .restaurants(true)
            .await?
            .iter()
            .any(|r| r.name.eq_ignore_ascii_case(name))
        {
            return Err(Error::Invalid(format!("{name} already exists")));
        }
        self.store.save_restaurant(&input, self.clock.now()).await
    }

    pub async fn restaurants(&self) -> Result<Vec<Restaurant>> {
        self.store.restaurants(false).await
    }
}
