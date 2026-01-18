use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    mem,
    num::{NonZeroI64, NonZeroU32, TryFromIntError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u32);

impl Timestamp {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn since(self, earlier: Timestamp) -> Result<Seconds, TimeError> {
        debug_assert!(self >= earlier, "timestamps must be ordered");

        if self == earlier {
            Err(TimeError::ZeroDuration)
        } else {
            Ok(Seconds(unsafe {
                // checked in assert_debug
                NonZeroU32::new_unchecked(self.0 - earlier.0)
            }))
        }
    }

    pub fn inner(&self) -> u32 {
        self.0
    }

    pub fn day(&self) -> u32 {
        self.0 / 86400
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TimeError {
    #[error("timestamps are equal")]
    ZeroDuration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MileMarker(u16);

impl MileMarker {
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    pub fn distance_to(self, other: MileMarker) -> Miles {
        let (max, min) = if self >= other {
            (self, other)
        } else {
            (other, self)
        };

        Miles(u32::from(max.0 - min.0))
    }

    pub fn inner(&self) -> u16 {
        self.0
    }
}

impl From<u16> for MileMarker {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Miles(u32);

impl Miles {
    pub fn per(self, t: Seconds) -> Result<MphX100, SpeedError> {
        let miles = i64::from(self.0);
        let secs = NonZeroI64::from(t.0);

        let v = miles
            .checked_mul(360_000)
            .and_then(|x| x.checked_div(secs.into()))
            .ok_or(SpeedError::Overflow)?;

        u16::try_from(v)
            .map(MphX100)
            .map_err(|_| SpeedError::Overflow)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SpeedError {
    #[error("overflow")]
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seconds(NonZeroU32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MphX100(u16);

impl MphX100 {
    pub fn new(value: u16) -> Self {
        Self(value)
    }

    pub fn inner(self) -> u16 {
        self.0
    }
}

impl TryFrom<u32> for MphX100 {
    type Error = TryFromIntError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Ok(Self::new(value.try_into()?))
    }
}

impl From<u16> for MphX100 {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

#[derive(Default)]
pub struct State {
    cameras: HashMap<CameraID, Camera>,
    roads: HashMap<RoadID, Road>,
}

impl State {
    pub fn add_camera(
        &mut self,
        id: CameraID,
        road_id: RoadID,
        mile_marker: MileMarker,
        speed_limit: MphX100,
    ) {
        self.roads
            .entry(road_id)
            .or_insert(Road::new(road_id, speed_limit));

        self.cameras.entry(id).or_insert(Camera {
            road_id,
            mile_marker,
        });
    }

    pub fn remove_camera(&mut self, id: CameraID) {
        self.cameras.remove(&id);
    }

    pub fn add_observation(&mut self, camera_id: CameraID, plate: String, timestamp: Timestamp) {
        let camera = self.cameras.get(&camera_id).unwrap();
        let road = self.roads.get_mut(&camera.road_id).unwrap();

        let observation = Observation {
            timestamp,
            mile: camera.mile_marker,
        };

        road.add_observation(plate, observation);
    }

    pub fn get_tickets_for_roads(&mut self, roads_ids: &HashSet<RoadID>) -> Vec<Ticket> {
        self.roads
            .iter_mut()
            .filter(|(road_id, _)| roads_ids.contains(road_id))
            .fold(Vec::new(), |mut tickets, (_, road)| {
                tickets.extend(road.take_tickets());
                tickets
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CameraID(String);

impl<T> From<T> for CameraID
where
    T: Display,
{
    fn from(value: T) -> Self {
        Self(value.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoadID(u16);

impl RoadID {
    pub fn inner(&self) -> u16 {
        self.0
    }
}

impl From<u16> for RoadID {
    fn from(value: u16) -> Self {
        Self(value)
    }
}

struct Camera {
    road_id: RoadID,
    mile_marker: MileMarker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observation {
    pub timestamp: Timestamp,
    pub mile: MileMarker,
}

impl PartialOrd for Observation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Observation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

#[derive(Debug, PartialEq)]
pub struct Ticket {
    pub plate: String,
    pub road_id: RoadID,
    pub observation1: Observation,
    pub observation2: Observation,
    pub speed: MphX100,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TicketLogEntry(String, u32);

struct Road {
    id: RoadID,
    limit: MphX100,
    observations: HashMap<String, Vec<Observation>>,
    tickets: Vec<Ticket>,
    tickets_log: HashSet<TicketLogEntry>,
}

impl Road {
    pub fn new(id: RoadID, limit: MphX100) -> Self {
        Self {
            id,
            limit,
            observations: HashMap::new(),
            tickets: Vec::new(),
            tickets_log: HashSet::new(),
        }
    }

    pub fn add_observation(&mut self, plate: String, observation: Observation) {
        let observations = self.observations.entry(plate.clone()).or_default();
        if let Err(index) = observations.binary_search(&observation) {
            observations.insert(index, observation);
        } else {
            return;
        }

        for observations_pair in observations.windows(2) {
            let (older, newer) = (&observations_pair[0], &observations_pair[1]);

            let seconds = newer.timestamp.since(older.timestamp).expect("0 seconds");
            let miles = newer.mile.distance_to(older.mile);
            let speed = match miles.per(seconds) {
                Ok(speed) => speed,
                Err(err) => {
                    tracing::error!("speed calculation error: {err}");
                    continue;
                }
            };

            if speed > self.limit {
                let (older_day, newer_day) = (older.timestamp.day(), newer.timestamp.day());
                let ticket_log_entries: Vec<_> = (older_day..=newer_day)
                    .map(|day| TicketLogEntry(plate.clone(), day))
                    .collect();

                if ticket_log_entries
                    .iter()
                    .any(|entry| self.tickets_log.contains(entry))
                {
                    continue;
                }

                self.tickets.push(Ticket {
                    plate: plate.clone(),
                    road_id: self.id,
                    observation1: *older,
                    observation2: *newer,
                    speed,
                });

                self.tickets_log.extend(ticket_log_entries);
            }
        }
    }

    pub fn take_tickets(&mut self) -> Vec<Ticket> {
        mem::take(&mut self.tickets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_observation_below_speed_limit() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(10),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(20),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());

        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(3600));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert!(tickets.is_empty());
    }

    #[test]
    fn test_add_observation_above_speed_limit() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(10),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(20),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());

        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(300));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 1);

        let ticket = &tickets[0];
        assert_eq!(ticket.plate, "foo");
        assert_eq!(ticket.road_id, 1.into());
        assert_eq!(ticket.observation1.timestamp, Timestamp::new(0));
        assert_eq!(ticket.observation1.mile, MileMarker::new(10));
        assert_eq!(ticket.observation2.timestamp, Timestamp::new(300));
        assert_eq!(ticket.observation2.mile, MileMarker::new(20));
        assert_eq!(ticket.speed, MphX100(120 * 100));
    }

    #[test]
    fn test_add_observation_below_and_above_speed_limit() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(10),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(20),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-3".into(),
            1.into(),
            MileMarker::new(30),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());

        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(3600));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());

        state.add_observation("camera-3".into(), "foo".into(), Timestamp::new(3900));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 1);

        let ticket = &tickets[0];
        assert_eq!(ticket.plate, "foo");
        assert_eq!(ticket.road_id, 1.into());
        assert_eq!(ticket.observation1.timestamp, Timestamp::new(3600));
        assert_eq!(ticket.observation1.mile, MileMarker::new(20));
        assert_eq!(ticket.observation2.timestamp, Timestamp::new(3900));
        assert_eq!(ticket.observation2.mile, MileMarker::new(30));
        assert_eq!(ticket.speed, MphX100(120 * 100));
    }

    #[test]
    fn test_add_observation_same_timestamp() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(10),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(20),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());

        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(0));
        assert!(state.get_tickets_for_roads(&roads_ids).is_empty());
    }

    #[test]
    fn test_ticket_is_per_road() {
        let mut state = State::default();

        state.add_camera(
            "camera-1a".into(),
            1.into(),
            MileMarker::new(0),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-1b".into(),
            1.into(),
            MileMarker::new(60),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2a".into(),
            2.into(),
            MileMarker::new(0),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2b".into(),
            2.into(),
            MileMarker::new(60),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1), RoadID::from(2)].into_iter().collect();

        state.add_observation("camera-1a".into(), "foo".into(), Timestamp::new(0));
        state.add_observation("camera-1b".into(), "foo".into(), Timestamp::new(1800));

        state.add_observation("camera-2a".into(), "foo".into(), Timestamp::new(0));
        state.add_observation("camera-2b".into(), "foo".into(), Timestamp::new(1800));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 2);
    }

    #[test]
    fn test_ticket_covers_all_days() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(0),
            MphX100(60 * 100),
        );
        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(200),
            MphX100(60 * 100),
        );
        state.add_camera(
            "camera-3".into(),
            1.into(),
            MileMarker::new(210),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        // This speeding ticket spans day 0 to day 1.
        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(86300));
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(97300));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 1);

        // Another speeding pair on day 1 should be suppressed.
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(97300));
        state.add_observation("camera-3".into(), "foo".into(), Timestamp::new(97600));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert!(tickets.is_empty());
    }

    #[test]
    fn test_ticket_covers_intermediate_days_for_long_span() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(0),
            MphX100(60 * 100),
        );
        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(47_000),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        // First ticket spans from day 0 to day 3.
        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(3 * 86_400));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 1);

        // Another speeding pair on day 1 should be suppressed.
        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(90_000));
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(90_600));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert!(tickets.is_empty());
    }

    #[test]
    fn test_speed_with_reverse_direction() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(20),
            MphX100(60 * 100),
        );

        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(10),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(300));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert_eq!(tickets.len(), 1);
    }

    #[test]
    fn test_add_observation_speed_overflow_is_ignored() {
        let mut state = State::default();

        state.add_camera(
            "camera-1".into(),
            1.into(),
            MileMarker::new(0),
            MphX100(60 * 100),
        );
        state.add_camera(
            "camera-2".into(),
            1.into(),
            MileMarker::new(u16::MAX),
            MphX100(60 * 100),
        );

        let roads_ids: HashSet<RoadID> = [RoadID::from(1)].into_iter().collect();

        state.add_observation("camera-1".into(), "foo".into(), Timestamp::new(0));
        state.add_observation("camera-2".into(), "foo".into(), Timestamp::new(1));

        let tickets = state.get_tickets_for_roads(&roads_ids);
        assert!(tickets.is_empty());
    }
}
