
use super::data::{
    EngineData,
    Stop,
    StopIdx,
    StopPatternIdx,
    VehicleData,
};

use super::iters::{ArrivalTimetablesOfStop};

use super::time::{ DaysSinceDatasetStart ,SecondsSinceDatasetStart, SecondsSinceDayStart};

use super::calendars::{DaysIter};

use super::ordered_timetable::{Timetable, Position, Vehicle, TimetableData, VehiclesIter};
use std::hash::Hash;
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ForwardMission {
    pub stop_pattern : StopPatternIdx,
    pub timetable : Timetable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ForwardTrip {
    pub mission : ForwardMission,
    pub vehicle : Vehicle,
    pub day : DaysSinceDatasetStart,
}



impl EngineData {


    pub fn is_upstream_in_forward_mission(&self,
        upstream : & StopIdx,
        downstream : & StopIdx,
        mission : & ForwardMission,
    ) -> bool {
        let pattern = self.arrival_pattern(&mission.stop_pattern);
        pattern.is_upstream(upstream, downstream)
       
    }

    pub fn next_stop_in_forward_mission(&self,
        stop : & StopIdx,
        mission : & ForwardMission,
    ) -> Option<StopIdx> 
    {
        let pattern = self.arrival_pattern(&mission.stop_pattern);
        pattern.next_stop(stop).cloned()
    }


    pub fn boardable_forward_missions<'a>(& 'a self, 
        stop_idx : & StopIdx
    ) -> ForwardMissionsOfStop
    {
        let inner = self.arrival_pattern_and_timetables_of(stop_idx);
        ForwardMissionsOfStop {
            inner
        }
    }

    pub fn forward_mission_of(&self, forward_trip : & ForwardTrip) -> ForwardMission {
        forward_trip.mission.clone()
    }

    pub fn forward_trips_of(&self, forward_mission : & ForwardMission) -> ForwardTripsOfMission {
        ForwardTripsOfMission::new(&self, forward_mission)
    }

    // Panics if `trip` does not go through `stop_idx` 
    pub fn arrival_time_of(&self, trip : & ForwardTrip, stop : & StopIdx) -> SecondsSinceDatasetStart {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = & trip.vehicle;
        let seconds_in_day = self.arrival_pattern(pattern).debark_time_at(timetable, vehicle, stop);
        let days = &trip.day;
        SecondsSinceDatasetStart::compose(days, seconds_in_day)
    }

    // Panics if `trip` does not go through `stop_idx` 
    // None if `trip` does not allows boarding at `stop_idx`
    pub fn departure_time_of(&self, trip : & ForwardTrip, stop : & StopIdx) -> Option<SecondsSinceDatasetStart> {
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = & trip.vehicle;
        let has_seconds_in_day = self.arrival_pattern(pattern).board_time_at(timetable, vehicle, stop);
        has_seconds_in_day.as_ref().map(|seconds_in_day| {
            let days = &trip.day;
            SecondsSinceDatasetStart::compose(days, &seconds_in_day)
        })
        
    }


    pub fn best_trip_to_board_at_stop(&self,
        waiting_time : & SecondsSinceDatasetStart,
        mission : & ForwardMission,
        stop : & StopIdx
     ) -> Option<(ForwardTrip, SecondsSinceDatasetStart)> 
     {
        let stop_pattern = &mission.stop_pattern;
        let timetable = &mission.timetable;
        self.best_vehicle_to_board(waiting_time, stop_pattern, timetable, stop)
            .map(|(vehicle, day, arrival_time)| {
                let trip = ForwardTrip {
                    mission : mission.clone(),
                    day,
                    vehicle,               
                };
                (trip, arrival_time)
            })

     }

    fn best_vehicle_to_board(&self, 
        waiting_time : & SecondsSinceDatasetStart,
        stop_pattern : & StopPatternIdx,
        timetable : & Timetable,
        stop : & StopIdx
     ) -> Option<(Vehicle, DaysSinceDatasetStart,SecondsSinceDatasetStart)> 
     {


        //TODO : reread this and look for optimization

        let pattern_data = self.arrival_pattern(stop_pattern);
        // we should never try to board a stop pattern at its last position
        debug_assert!(! pattern_data.is_last_stop(stop));

        

        let has_latest_board_time = pattern_data.last_board_time_at(timetable, stop);
        if has_latest_board_time.is_none() {
            return None;
        }
        let latest_board_time_in_day = has_latest_board_time.clone().unwrap();

        let mut nb_of_days_to_offset = 0u16;
        let (mut waiting_day, mut waiting_time_in_day) = waiting_time.decompose();
        let mut best_vehicle_day_and_its_debark_time_at_next_stop : Option<(Vehicle, DaysSinceDatasetStart, SecondsSinceDatasetStart)> = None;

        let position = pattern_data.position(stop);
        let next_stop = pattern_data.next_stop(stop).unwrap();
        let next_position = pattern_data.position(next_stop);

        let timetable_data = pattern_data.timetable_data(timetable);
        
        while waiting_time_in_day <= latest_board_time_in_day {
            
            let has_vehicle  = self.best_vehicle_to_board_in_day(&waiting_day, 
                &waiting_time_in_day, 
                timetable_data, 
                position
            );
            if let Some(vehicle) = has_vehicle {
                let vehicle_debark_time_in_day_at_next_stop = timetable_data.debark_time_at(&vehicle, &next_position);
                let vehicle_debark_time_at_next_stop = SecondsSinceDatasetStart::compose(&waiting_day, vehicle_debark_time_in_day_at_next_stop);
                if let Some((_, _, best_debark_time)) = & best_vehicle_day_and_its_debark_time_at_next_stop {
                    if vehicle_debark_time_at_next_stop < *best_debark_time {
                        best_vehicle_day_and_its_debark_time_at_next_stop = Some((vehicle, waiting_day, vehicle_debark_time_at_next_stop));
                    }
                }
                else {
                    best_vehicle_day_and_its_debark_time_at_next_stop = Some((vehicle, waiting_day, vehicle_debark_time_at_next_stop));
                }

            }
            nb_of_days_to_offset += 1;
            let has_prev_day = waiting_time.decompose_with_days_offset(nb_of_days_to_offset);
            if let Some((day, time_in_day)) = has_prev_day {
                waiting_day = day;
                waiting_time_in_day = time_in_day;
            }
            else {
                break;
            }
        }

        best_vehicle_day_and_its_debark_time_at_next_stop
       
    }

    fn best_vehicle_to_board_in_day(&self, 
        day : & DaysSinceDatasetStart,
        time_in_day : & SecondsSinceDayStart,
        timetable : & TimetableData<VehicleData, SecondsSinceDayStart>,
        position : & Position,
    ) -> Option<Vehicle>
    {
        timetable.best_filtered_vehicle_to_board_at(time_in_day, position, |vehicle_data| {
            let calendar_idx = vehicle_data.calendar_idx;
            self.calendars.is_allowed(&calendar_idx, day)
        })

    }

}


pub struct ForwardMissionsOfStop<'a> {
    inner : ArrivalTimetablesOfStop<'a>
}

impl<'a> Iterator for ForwardMissionsOfStop<'a> {
    type Item = ForwardMission;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(pattern, timetable)| {
            ForwardMission{
                stop_pattern : pattern,
                timetable
            }
        })
    }
}

pub struct ForwardTripsOfMission {
    mission : ForwardMission,
    has_current_vehicle : Option<Vehicle>, // None when the iterator is exhausted
    vehicles_iter : VehiclesIter,
    days_iter : DaysIter,
}

impl ForwardTripsOfMission {
    fn new(engine_data : & EngineData, mission : & ForwardMission) -> Self {
        let pattern_idx = mission.stop_pattern.idx;
        let stop_pattern = & engine_data.arrival_stop_patterns[pattern_idx];
        let timetable = stop_pattern.timetable_data(&mission.timetable);

        let mut vehicles_iter = timetable.vehicles();
        let has_current_vehicle = vehicles_iter.next();
        let days_iter = engine_data.calendars.days();

        Self {
            mission : mission.clone(),
            has_current_vehicle,
            vehicles_iter,
            days_iter
        }

    }
}

impl Iterator for ForwardTripsOfMission {
    type Item = ForwardTrip;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(current_vehicle) = & mut self.has_current_vehicle {
                match self.days_iter.next() {
                    Some(day) => {
                        let trip = 
                        ForwardTrip {
                            mission : self.mission.clone(),
                            vehicle : current_vehicle.clone(),
                            day,
                        };
                        return Some(trip);
                    },
                    None => {
                        self.has_current_vehicle = self.vehicles_iter.next();
                    }
                }

            }
            else {
                return None;
            }
        }
    }
}