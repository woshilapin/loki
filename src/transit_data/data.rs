
use transit_model;
use transit_model::{
    model::Model as TransitModel,
    objects::{StopPoint, VehicleJourney, Transfer as TransitModelTransfer},

}; 
pub(super) use transit_model::objects::Time as TransitModelTime;


use std::path::PathBuf;
use std::collections::{BTreeMap};
use super::ordered_timetable::{StopPatternData, Position, Timetable, Vehicle};
use super::calendars::{Calendars, CalendarIdx};
use super::time::{SecondsSinceDayStart, PositiveDuration, DaysSinceDatasetStart};
use typed_index_collection::{Idx};

use crate::request::response::{Journey, VehicleSection};

use std::collections::HashMap;

use log::info;

#[derive(Debug, Copy, Clone)]
pub struct Duration {
    pub (super) seconds : u32
}


#[derive(Debug, Clone)]
pub struct VehicleData {
    pub (super) vehicle_journey_idx : Idx<VehicleJourney>,
    pub (super) calendar_idx : CalendarIdx,

}

pub struct StopData {
    pub (super) stop_point_idx : Idx<StopPoint>,
    pub (super) position_in_patterns : Vec<(StopPattern, Position)>,
    pub (super) transfers : Vec<(Stop, PositiveDuration, Option<Idx<TransitModelTransfer>>)>
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Ord, PartialOrd)]
pub enum FlowDirection{
    BoardOnly,
    DebarkOnly,
    BoardAndDebark,
}
pub type StopPoints = Vec< (Idx<StopPoint>, FlowDirection) >;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct StopPattern {
    pub (super) idx : usize
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct Stop {
    pub (super) idx : usize
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub (super) stop : Stop,
    pub (super) idx_in_stop_transfers : usize,
}


pub struct TransitData {
    pub (super) stop_points_to_pattern : BTreeMap< StopPoints, StopPattern>,
    pub (super) stop_point_idx_to_stop : HashMap< Idx<StopPoint>, Stop  >,

    pub (super) stops_data : Vec<StopData>,
    pub (super) patterns : Vec<StopPatternData>,

    pub (super) calendars : Calendars,


}


#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Mission {
    pub stop_pattern : StopPattern,
    pub timetable : Timetable,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Trip {
    pub mission : Mission,
    pub vehicle : Vehicle,
    pub day : DaysSinceDatasetStart,
}



impl TransitData {
    pub fn stop_data<'a>(& 'a self, stop : & Stop) -> & 'a StopData {
        & self.stops_data[stop.idx]
    }

    pub fn pattern<'a>(& 'a self, pattern : & StopPattern) -> & 'a StopPatternData {
        & self.patterns[pattern.idx]
    }

    pub fn transfer(&self, stop : & Stop, transfer : & Transfer) -> (Stop, PositiveDuration) {
        debug_assert!(*stop == transfer.stop);
        let stop_data = self.stop_data(stop);
        let result = stop_data.transfers[transfer.idx_in_stop_transfers];
        (result.0, result.1)
    }

    pub fn nb_of_stops(&self) -> usize {
        self.stops_data.len()
    }

    pub fn stop_to_usize(&self, stop : & Stop) -> usize {
        stop.idx
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx : & Idx<StopPoint>) -> Option<&Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx)
    }

    pub fn nb_of_patterns(&self) -> usize {
        self.patterns.len()
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.patterns.iter().map(|pattern| {
            pattern.nb_of_timetables()
        }).sum()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.patterns.iter().map(|pattern| {
            pattern.nb_of_vehicles()
        }).sum()
    }

    pub fn print_response(&self, response : & Journey, transit_model : & TransitModel) -> () {
        info!("*** New journey ***");
        info!("Arrival time {:?}", response.arrival_section.to_datetime);
        let mut transfer_duration = PositiveDuration::zero();
        for (transfer_section, _, _) in response.connections.iter() {
            let stop = &transfer_section.from_stop;
            let transfer = &transfer_section.transfer;
            transfer_duration = transfer_duration + self.stops_data[stop.idx].transfers[transfer.idx_in_stop_transfers].1.clone();
        }

        info!("Transfer duration {:?}", transfer_duration);
        info!("Nb of vehicles : {}", 1 + response.connections.len());
        
        info!("Departure {}", self.calendars.to_string(&response.departure_section.from_datetime));

        self.print_vehicle_section(&response.first_vehicle, transit_model);
        for connection in response.connections.iter() {
            self.print_vehicle_section(&connection.2, transit_model);
        }
    }

    fn print_vehicle_section(&self, vehicle_section : & VehicleSection, transit_model : & TransitModel) {
        let trip = &vehicle_section.trip;
        let pattern = &trip.mission.stop_pattern;
        let timetable = &trip.mission.timetable;
        let vehicle = &trip.vehicle;
        let vehicle_journey_idx = self.patterns[pattern.idx].vehicle_data(timetable, vehicle).vehicle_journey_idx;
        let route_id = &transit_model.vehicle_journeys[vehicle_journey_idx].route_id;
        let route_id = &transit_model.routes.get(route_id).unwrap().id;
        let from_stop_idx = &self.stops_data[vehicle_section.from_stop.idx].stop_point_idx;
        let to_stop_idx = &self.stops_data[vehicle_section.to_stop.idx].stop_point_idx;
        let from_stop_id = &transit_model.stop_points[*from_stop_idx].id;
        let to_stop_id = &transit_model.stop_points[*to_stop_idx].id;
        let from_datetime = self.calendars.to_string(&vehicle_section.from_datetime);
        let to_datetime = self.calendars.to_string(&vehicle_section.to_datetime);
        info!("{} from {} at {} to {} at {} ", 
            route_id, 
            from_stop_id,
            from_datetime,
            to_stop_id,
            to_datetime
        );

    }
}






