use std::cmp::Ordering;
use super::data::{Stop, FlowDirection};
use std::ops::Range;
use std::iter::Map;
use std::collections::HashMap;

use super::time::SecondsSinceDayStart as Time;

// TODO : document more explicitely !
pub struct StopPatternData<VehicleData> {
    pub stops : Vec<Stop>,
    pub flow_directions : Vec<FlowDirection>,
    pub timetables : Vec<TimetableData<VehicleData>>,
}


// TODO : document more explicitely !
pub struct TimetableData<VehicleData> {
    // vehicles data, ordered by increasing times
    // meaning that is v1 is before v2 in this vector,
    // then for all `position` we have 
    //    debark_time_by_vehicle[v1][position] <= debark_time_by_vehicle[v2][position]
    vehicles_data : Vec<VehicleData>,


    // `board_times_by_position[position][vehicle]`
    //   is the time at which a traveler waiting
    //   at `position` can board `vehicle`
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  board_times_by_position[position] is sorted by increasing times
    board_times_by_position : Vec<Vec<Time>>, 

    // `debark_times_by_position[position][vehicle]`
    //   is the time at which a traveler being inside `vehicle`
    //   will debark at `position` 
    // Vehicles are ordered by increasing time
    //  so for each `position` the vector
    //  debark_times_by_position[position] is sorted by increasing times
    debark_times_by_position : Vec<Vec<Time>>, 

    latest_board_time_by_position : Vec<Time>, 


}

#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd)]
pub struct Position {
    idx : usize,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct Timetable {
    idx : usize,
}
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Vehicle {
    idx : usize
}


pub type TimetablesIter = Map<Range<usize>, fn(usize) -> Timetable >;

impl<VehicleData> StopPatternData<VehicleData>
{
    pub fn new(stops : Vec<Stop>, flow_directions : Vec<FlowDirection>) -> Self {
        assert!(stops.len() >= 2);
        assert!(flow_directions.len() == stops.len());
        assert!(flow_directions.first().unwrap() == &FlowDirection::BoardOnly);
        assert!(flow_directions.last().unwrap() == &FlowDirection::DebarkOnly);
        Self{
            stops,
            flow_directions,
            timetables : Vec::new(),
        }
    }


    pub fn stops_and_positions(&self) -> impl Iterator<Item = (Stop, Position)> + '_ {
        self.stops.iter().enumerate()
            .map(|(idx, stop)| {
                let position = Position { idx };
                (stop.clone(), position)
            })
    }

    pub fn is_valid(&self, position : & Position) -> bool {
        position.idx < self.nb_of_positions()
    }

    pub fn next_position(&self, position : & Position) -> Option<Position> {
        let next_position = Position{ idx : position.idx + 1};
        if self.is_valid(&next_position) {
            Some(next_position)
        }
        else{
            None
        }
    }

    pub fn stop_at(&self, position : & Position) -> & Stop {
        & self.stops[position.idx]
    }


    pub fn is_last_position(&self, position : & Position) -> bool {
        position.idx == self.stops.len() - 1   
    }


    pub fn can_debark(&self, position : & Position) -> bool {
        let flow_direction = & self.flow_directions[position.idx];
        match flow_direction {
            FlowDirection::BoardAndDebark 
            | FlowDirection::DebarkOnly => { true},
            FlowDirection::BoardOnly => { false }
        }
    }

    pub fn can_board(&self, position : & Position) -> bool {
        let flow_direction = & self.flow_directions[position.idx];
        match flow_direction {
            FlowDirection::BoardAndDebark 
            | FlowDirection::BoardOnly => { true},
            FlowDirection::DebarkOnly => { false }
        }
    }

    fn timetable_data<'a>(& 'a self, timetable : & Timetable) -> & 'a TimetableData<VehicleData> {
        self.timetables.get(timetable.idx)
            .unwrap_or_else( || panic!(format!(
                "The timetable {:?} is expected to belongs to the stop_pattern ", 
                    *timetable)
                )
            )
    }

    pub fn timetables(&self) -> TimetablesIter {
        (0..self.timetables.len()).map(|idx| {
            Timetable {
                idx
            }
        })
    }

    pub fn nb_of_timetables(&self) -> usize {
        self.timetables.len()
    }

    pub fn vehicles(&self, timetable : & Timetable) -> VehiclesIter {
        let timetable_data = self.timetable_data(timetable);
        let nb_of_vehicles = timetable_data.nb_of_vehicles();
        (0..nb_of_vehicles).map(|idx| {
            Vehicle{
                idx
            }
        })
    }

    pub fn debark_time_at(&self, timetable : & Timetable, vehicle : & Vehicle, position : & Position) -> Option<& Time> {
        
        if self.can_debark(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.debark_time_at(vehicle, position);
            Some(time)
        }
        else {
            None
        }
    }

    pub fn arrival_time_at(&self, timetable : & Timetable, vehicle : & Vehicle, position : & Position) -> & Time {
        
        let timetable_data = self.timetable_data(timetable);
        timetable_data.debark_time_at(vehicle, position)

    }

    pub fn board_time_at(&self, timetable : & Timetable, vehicle: & Vehicle, position : & Position) -> Option<&Time> {
        if self.can_board(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.board_time_at(vehicle, position);
            Some(time)
        }
        else {
            None
        }
    }

    pub fn latest_board_time_at(&self, timetable : & Timetable, position : & Position) -> Option<& Time> {
        if self.can_board(position) {
            let timetable_data = self.timetable_data(timetable);
            let time = timetable_data.latest_board_time_at(position);
            Some(time)
        }
        else {
            None
        }
    }

    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // will return the index of the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time.
    pub fn earliest_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        timetable : & Timetable,
        position : & Position,
        filter : Filter
    ) -> Option<Vehicle> 
    where Filter : Fn(&VehicleData) -> bool
    {
        if ! self.can_board(position) {
            return None;
        }
        let timetable_data = self.timetable_data(timetable);
        timetable_data.best_filtered_vehicle_to_board_at(waiting_time, position, filter)
    }

    fn nb_of_positions(&self) -> usize {
        self.stops.len()
    }

    // Insert in the vehicle in a timetable if 
    // the given debark_times and board_times are coherent.
    // Returns a VehicleTimesError otherwise.
    pub fn insert<'a, 'b, BoardDebarkTimes,  >(& mut self,  
        board_debark_times : BoardDebarkTimes, 
        vehicle_data : VehicleData
    ) -> Result<(), VehicleTimesError>
    where 
    BoardDebarkTimes : Iterator<Item = (Time, Time)> + ExactSizeIterator + Clone,
    {
        assert!(self.nb_of_positions() == board_debark_times.len());

        let valid_enumerated_board_times = board_debark_times.clone()
                                .zip(self.flow_directions.iter())
                                .enumerate()
                                .filter_map(|(position, ((board_time, _), flow_direction)) | {
                                    match flow_direction {
                                        FlowDirection::BoardOnly 
                                        | FlowDirection::BoardAndDebark => { Some((position, board_time))},
                                        FlowDirection::DebarkOnly => { None },
                                    }
                                });

        if let Err((upstream, downstream)) = is_increasing(valid_enumerated_board_times.clone()) {
            let position_pair = PositionPair{
                upstream,
                downstream
            };
            return Err(VehicleTimesError::DecreasingBoardTime(position_pair));
        }


        let valid_enumerated_debark_times = board_debark_times.clone()
                .zip(self.flow_directions.iter())
                .enumerate()
                .filter_map(|(position, ((_, debark_time), flow_direction)) | {
                    match flow_direction {
                        FlowDirection::DebarkOnly 
                        | FlowDirection::BoardAndDebark => { Some((position, debark_time))},
                        FlowDirection::BoardOnly => { None },
                    }
                });

        if let Err((upstream, downstream)) = is_increasing(valid_enumerated_debark_times.clone()) {
            let position_pair = PositionPair{
                upstream,
                downstream
            };
            return Err(VehicleTimesError::DecreasingDebarkTime(position_pair))
        }

        let pair_iter = board_debark_times.clone().zip(board_debark_times.clone().skip(1)).enumerate();
        for (board_idx, ((board_time,_), (_, debark_time))) in pair_iter {
            let board_position = Position {
                idx : board_idx
            };
            let debark_position = Position {
                idx : board_idx + 1
            };
            if self.can_board(&board_position) && self.can_debark(&debark_position) && board_time > debark_time {
                let position_pair = PositionPair {
                    upstream : board_position.idx,
                    downstream : debark_position.idx
                };
                return Err(VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair));
            }
        }

        let corrected_board_debark_times = board_debark_times
            .zip(self.flow_directions.iter())
            .map(|((board_time, debark_time), flow_direction)| {
                match flow_direction {
                    FlowDirection::BoardAndDebark => (board_time, debark_time),
                    FlowDirection::BoardOnly => (board_time.clone(), board_time),
                    FlowDirection::DebarkOnly => (debark_time.clone(), debark_time)
                }
            });

        for timetable_data in & mut self.timetables {
            if timetable_data.accept(corrected_board_debark_times.clone()) {
                timetable_data.insert(corrected_board_debark_times, vehicle_data);
                return Ok(());
            }
        }
        let mut new_timetable_data =TimetableData::new(self.nb_of_positions());
        new_timetable_data.insert(corrected_board_debark_times, vehicle_data);
        self.timetables.push(new_timetable_data);
        Ok(())
    }

}

pub type VehiclesIter = Map<Range<usize>, fn(usize)->Vehicle>;

impl<VehicleData> TimetableData<VehicleData>
{

    fn new(nb_of_positions : usize) -> Self {
        assert!( nb_of_positions >= 2);
        Self{
            vehicles_data : Vec::new(),
            debark_times_by_position : vec![Vec::new(); nb_of_positions],
            board_times_by_position : vec![Vec::new(); nb_of_positions],
            latest_board_time_by_position : vec![Time::zero(); nb_of_positions],
        }
    }

    fn debark_time_at(&self, vehicle : & Vehicle, position :  & Position) -> &Time {
        &self.debark_times_by_position[position.idx][vehicle.idx]
    }

    fn board_time_at(&self, vehicle : & Vehicle, position :  & Position) -> &Time {
        & self.board_times_by_position[position.idx][vehicle.idx]
    }

    fn latest_board_time_at(&self, position : & Position) -> &Time {
        & self.latest_board_time_by_position[position.idx]
    }


    fn nb_of_positions(&self) -> usize {
        self.board_times_by_position.len()
    }

    fn nb_of_vehicles(&self) -> usize {
        self.vehicles_data.len()
    }

    fn vehicle_debark_times<'a>(& 'a self, vehicle_idx : usize) -> VehicleTimes<'a> {
        debug_assert!( vehicle_idx < self.vehicles_data.len() );
        VehicleTimes {
            times_by_position : & self.debark_times_by_position,
            position : 0,
            vehicle : vehicle_idx
        }
    }

    fn vehicle_board_times<'a>(& 'a self, vehicle_idx : usize) -> VehicleTimes<'a> {
        debug_assert!( vehicle_idx < self.vehicles_data.len() );
        VehicleTimes {
            times_by_position : & self.board_times_by_position,
            position : 0,
            vehicle : vehicle_idx
        }
    }


    fn accept<BoardDebarkTimes>(& self, board_debark_times : BoardDebarkTimes) -> bool 
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {
        assert!( board_debark_times.len() == self.nb_of_positions() );
        let board_times = board_debark_times.clone().map(|(board_time, _)| board_time);
        let debark_times = board_debark_times.map(|(_, debark_time)| debark_time);
        for vehicle_idx in 0..self.nb_of_vehicles() {
            let vehicle_debark_times = self.vehicle_debark_times(vehicle_idx);
            let debark_comparison = partial_cmp(vehicle_debark_times, debark_times.clone());
            if debark_comparison.is_none() {
                return false;
            }
            let vehicle_board_times = self.vehicle_board_times(vehicle_idx);
            let board_comparison = partial_cmp(vehicle_board_times, board_times.clone());
            if board_comparison.is_none() {
                return false;
            }

            match (board_comparison, debark_comparison) {
                 (Some(Ordering::Greater), Some(Ordering::Less)) 
                | (Some(Ordering::Less), Some(Ordering::Greater)) => {
                    return false;
                }
                _ => {()}
            }
        }
        true
    }

    fn insert<BoardDebarkTimes >(& mut self,  
            board_debark_times : BoardDebarkTimes, 
            vehicle_data : VehicleData)
    where 
    BoardDebarkTimes : Iterator<Item =  (Time, Time)> + ExactSizeIterator + Clone,
    {
        assert!(board_debark_times.len() == self.nb_of_positions());
        debug_assert!(self.accept(board_debark_times.clone()));
        let board_times = board_debark_times.clone().map(|(board_time, _)| board_time);
        let debark_times = board_debark_times.clone().map(|(_, debark_time)| debark_time);
        let nb_of_vehicles = self.nb_of_vehicles();
        // TODO : maybe start testing from the end ?
        // TODO : can be simplified if we know that self.accept(&debark_times) ??
        let insert_idx = (0..nb_of_vehicles).find(|&vehicle_idx| {
            let vehicle_debark_times = self.vehicle_debark_times(vehicle_idx);
            let debark_comparison = partial_cmp(vehicle_debark_times, debark_times.clone());
            let vehicle_board_times = self.vehicle_board_times(vehicle_idx);
            let board_comparison = partial_cmp(vehicle_board_times, board_times.clone());
            match (board_comparison, debark_comparison) {
                  (Some(Ordering::Less), Some(Ordering::Less))
                | (Some(Ordering::Equal), Some(Ordering::Less))
                | (Some(Ordering::Less), Some(Ordering::Equal))
                | (Some(Ordering::Equal), Some(Ordering::Equal)) => {true},
                _ => { false}
            }
        })
        .unwrap_or(nb_of_vehicles);

        for (position, (board_time, debark_time)) in board_debark_times.enumerate() {
            self.board_times_by_position[position].insert(insert_idx, board_time.clone());
            self.debark_times_by_position[position].insert(insert_idx, debark_time);
            let latest_board_time = & mut self.latest_board_time_by_position[position];
            * latest_board_time = std::cmp::max(latest_board_time.clone(), board_time);

        }
        self.vehicles_data.insert(insert_idx, vehicle_data);

    }


    // If we are waiting to board a vehicle at `position` at time `waiting_time`
    // return `Some(best_vehicle)`
    // where `best_vehicle` is the vehicle, among those vehicle on which `filter` returns true,
    //  to board that allows to debark at the subsequent positions at the earliest time,
    fn best_filtered_vehicle_to_board_at<Filter>(&self, 
        waiting_time : & Time, 
        position : & Position,
        filter : Filter
    ) -> Option<Vehicle> 
    where Filter : Fn(&VehicleData) -> bool
    {
        self.board_times_by_position[position.idx].iter()
            .zip(self.vehicles_data.iter())
            .enumerate()
            .filter(|(_, (_, vehicle_data)) | filter(vehicle_data))
            .find_map(|(idx, (board_time, _)) | {
                if waiting_time <= board_time {
                    let vehicle = Vehicle { idx };
                    Some(vehicle)
                }
                else {
                    None
                }
            })
    }

}



pub struct PositionPair {
    pub upstream : usize,
    pub downstream : usize,
}

pub enum VehicleTimesError {
    DebarkBeforeUpstreamBoard(PositionPair), // board_time[upstream] > debark_time[downstream]
    DecreasingBoardTime(PositionPair),  // board_time[upstream] > board_time[downstream] 
    DecreasingDebarkTime(PositionPair)  // debark_time[upstream] > debark_time[downstream] 
}

fn is_increasing<EnumeratedValues>(mut enumerated_values : EnumeratedValues) -> Result<(), (usize, usize) >
where EnumeratedValues : Iterator<Item = (usize, Time)>
{
    let has_previous = enumerated_values.next();
    let (mut prev_position, mut prev_value) = has_previous.unwrap();
    for (position, value) in enumerated_values {
        if value < prev_value {
            return Err((prev_position, position));
        }
        prev_position = position;
        prev_value = value;
    }
    Ok(())
}

// Retuns 
//    - Some(Equal)   if lower[i] == upper[i] for all i
//    - Some(Less)    if lower[i] <= upper[i] for all i
//    - Some(Greater) if lower[i] >= upper[i] for all i
//    - None otherwise (the two vector are not comparable)
fn partial_cmp<Lower, Upper, Value> (lower : Lower, upper : Upper) -> Option<Ordering> 
where 
Lower : Iterator<Item = Value> + Clone,
Upper : Iterator<Item = Value> + Clone,
Value : Ord,
{
    assert!( lower.clone().count() == upper.clone().count() );
    let zip_iter = lower.zip(upper);
    let mut first_not_equal_iter = zip_iter.skip_while(|(lower_val, upper_val) | lower_val == upper_val);
    let has_first_not_equal = first_not_equal_iter.next();
    if let Some(first_not_equal) = has_first_not_equal {
        let ordering = {
            let lower_val = first_not_equal.0;
            let upper_val = first_not_equal.1;
            lower_val.cmp(&upper_val)
        };
        debug_assert!( ordering != Ordering::Equal);
        // let's see if there is an inder where the ordering is not the same
        // as first_ordering
        let found = first_not_equal_iter.find(|(lower_val, upper_val)| {
            let cmp = lower_val.cmp(&upper_val);
            cmp != ordering && cmp != Ordering::Equal
        });
        if found.is_some() {
            return None;
        }
        // if found.is_none(), it means that 
        // all elements are ordered the same, so the two vectors are comparable
        return Some(ordering);
    }
    // if has_first_not_equal == None
    // then values == item_values
    // the two vector are equal
    return Some(Ordering::Equal);
    
}
#[derive(Clone)]
struct VehicleTimes<'a> {
    times_by_position : & 'a [Vec<Time>],
    position : usize,
    vehicle : usize
}

impl<'a> Iterator for VehicleTimes<'a> {
    type Item =  Time;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.times_by_position.get(self.position)
            .map( |time_by_vehicles| {
                &time_by_vehicles[self.vehicle]
            });
        if result.is_some() {
            self.position += 1;
        }    
        result.cloned()

    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.times_by_position.len() - self.position;
        (remaining, Some(remaining))
    }
}

impl<'a> ExactSizeIterator for VehicleTimes<'a> {

}

