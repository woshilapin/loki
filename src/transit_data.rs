// Copyright  (C) 2020, Kisio Digital and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
// the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
// powered by Kisio Digital (www.kisio.com).
// Help us simplify mobility and open public transport:
// a non ending quest to the responsive locomotion way of traveling!
//
// This contribution is a part of the research and development work of the
// IVA Project which aims to enhance traveler information and is carried out
// under the leadership of the Technological Research Institute SystemX,
// with the partnership and support of the transport organization authority
// Ile-De-France Mobilités (IDFM), SNCF, and public funds
// under the scope of the French Program "Investissements d’Avenir".
//
// LICENCE: This program is free software; you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
//
// Stay tuned using
// twitter @navitia
// channel `#navitia` on riot https://riot.im/app/#/room/#navitia:matrix.org
// https://groups.google.com/d/forum/navitia
// www.navitia.io
pub mod data_interface;
pub mod data_update;
pub mod init;
pub mod iters;

use chrono::NaiveDate;
use iters::MissionsOfStop;
pub use transit_model::objects::{
    StopPoint, Time as TransitModelTime, Transfer as TransitModelTransfer, VehicleJourney,
};
pub use typed_index_collection::Idx;

use crate::{
    loads_data::{Load, LoadsData},
    model::{ModelRefs, StopPointIdx, TransferIdx, VehicleJourneyIdx},
    time::{Calendar, PositiveDuration, SecondsSinceDatasetUTCStart},
    timetables::{
        generic_timetables::{PositionPair, VehicleTimesError},
        InsertionError,
    },
};

use std::{collections::HashMap, fmt::Debug};

use crate::timetables::{RemovalError, Timetables as TimetablesTrait, TimetablesIter};

use crate::transit_model::Model;

use crate::tracing::error;

pub struct TransitData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx_to_stop: HashMap<StopPointIdx, Stop>,

    pub(super) stops_data: Vec<StopData<Timetables>>,
    pub(super) timetables: Timetables,

    pub(super) transfers_data: Vec<TransferData>,

    pub(super) start_date: NaiveDate,
    pub(super) end_date: NaiveDate,
}

pub struct StopData<Timetables: TimetablesTrait> {
    pub(super) stop_point_idx: StopPointIdx,
    pub(super) position_in_timetables: Vec<(Timetables::Mission, Timetables::Position)>,
    pub(super) outgoing_transfers: Vec<(Stop, TransferDurations, Transfer)>,
    pub(super) incoming_transfers: Vec<(Stop, TransferDurations, Transfer)>,
}

#[derive(Debug, Clone)]
pub struct TransferDurations {
    pub walking_duration: PositiveDuration,
    pub total_duration: PositiveDuration, // = walking_duration + some waiting time
}

pub struct TransferData {
    pub from_stop: Stop,
    pub to_stop: Stop,
    pub durations: TransferDurations,
    pub transit_model_transfer_idx: TransferIdx,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash, Ord, PartialOrd)]
pub struct Stop {
    pub(super) idx: usize,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Transfer {
    pub(super) idx: usize,
}

impl<Timetables: TimetablesTrait> TransitData<Timetables> {
    pub fn stop_data(&self, stop: &Stop) -> &StopData<Timetables> {
        &self.stops_data[stop.idx]
    }

    pub fn stop_point_idx_to_stop(&self, stop_point_idx: &StopPointIdx) -> Option<&Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx)
    }
}

impl<Timetables: TimetablesTrait> data_interface::TransitTypes for TransitData<Timetables> {
    type Stop = Stop;
    type Mission = Timetables::Mission;
    type Position = Timetables::Position;
    type Trip = Timetables::Trip;
    type Transfer = Transfer;
}

impl<Timetables: TimetablesTrait> data_interface::Data for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn is_upstream(
        &self,
        upstream: &Self::Position,
        downstream: &Self::Position,
        mission: &Self::Mission,
    ) -> bool {
        self.timetables
            .is_upstream_in_mission(upstream, downstream, mission)
    }

    fn next_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.next_position(position, mission)
    }

    fn previous_on_mission(
        &self,
        position: &Self::Position,
        mission: &Self::Mission,
    ) -> Option<Self::Position> {
        self.timetables.previous_position(position, mission)
    }

    fn mission_of(&self, trip: &Self::Trip) -> Self::Mission {
        self.timetables.mission_of(trip)
    }

    fn stop_of(&self, position: &Self::Position, mission: &Self::Mission) -> Self::Stop {
        self.timetables.stop_at(position, mission)
    }

    fn board_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.board_time_of(trip, position)
    }

    fn debark_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> Option<(SecondsSinceDatasetUTCStart, Load)> {
        self.timetables.debark_time_of(trip, position)
    }

    fn arrival_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables.arrival_time_of(trip, position)
    }

    fn departure_time_of(
        &self,
        trip: &Self::Trip,
        position: &Self::Position,
    ) -> (SecondsSinceDatasetUTCStart, Load) {
        self.timetables.departure_time_of(trip, position)
    }

    fn transfer_from_to_stop(&self, transfer: &Self::Transfer) -> (Self::Stop, Self::Stop) {
        let transfer_data = &self.transfers_data[transfer.idx];
        (transfer_data.from_stop, transfer_data.to_stop)
    }

    fn transfer_duration(&self, transfer: &Self::Transfer) -> PositiveDuration {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.durations.total_duration
    }

    fn transfer_idx(&self, transfer: &Self::Transfer) -> TransferIdx {
        let transfer_data = &self.transfers_data[transfer.idx];
        transfer_data.transit_model_transfer_idx.clone()
    }

    fn earliest_trip_to_board_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .earliest_trip_to_board_at(waiting_time, mission, position)
    }

    fn earliest_filtered_trip_to_board_at<Filter>(
        &self,
        waiting_time: &SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        self.timetables
            .earliest_filtered_trip_to_board_at(waiting_time, mission, position, filter)
    }

    fn latest_trip_that_debark_at(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)> {
        self.timetables
            .latest_trip_that_debark_at(waiting_time, mission, position)
    }

    fn latest_filtered_trip_that_debark_at<Filter>(
        &self,
        waiting_time: &crate::time::SecondsSinceDatasetUTCStart,
        mission: &Self::Mission,
        position: &Self::Position,
        filter: Filter,
    ) -> Option<(Self::Trip, SecondsSinceDatasetUTCStart, Load)>
    where
        Filter: Fn(&VehicleJourneyIdx) -> bool,
    {
        self.timetables
            .latest_filtered_trip_that_debark_at(waiting_time, mission, position, filter)
    }

    fn to_naive_datetime(
        &self,
        seconds: &crate::time::SecondsSinceDatasetUTCStart,
    ) -> chrono::NaiveDateTime {
        self.timetables.calendar().to_naive_datetime(seconds)
    }

    fn vehicle_journey_idx(&self, trip: &Self::Trip) -> VehicleJourneyIdx {
        self.timetables.vehicle_journey_idx(trip)
    }

    fn stop_point_idx(&self, stop: &Stop) -> StopPointIdx {
        self.stops_data[stop.idx].stop_point_idx.clone()
    }

    fn stoptime_idx(&self, position: &Self::Position, trip: &Self::Trip) -> usize {
        self.timetables.stoptime_idx(position, trip)
    }

    fn day_of(&self, trip: &Self::Trip) -> chrono::NaiveDate {
        self.timetables.day_of(trip)
    }

    fn is_same_stop(&self, stop_a: &Self::Stop, stop_b: &Self::Stop) -> bool {
        stop_a.idx == stop_b.idx
    }

    fn calendar(&self) -> &Calendar {
        self.timetables.calendar()
    }

    fn stop_point_idx_to_stop(&self, stop_point_idx: &StopPointIdx) -> Option<Self::Stop> {
        self.stop_point_idx_to_stop.get(stop_point_idx).copied()
    }

    fn nb_of_trips(&self) -> usize {
        self.timetables.nb_of_trips()
    }

    fn nb_of_stops(&self) -> usize {
        self.stops_data.len()
    }

    fn stop_id(&self, stop: &Stop) -> usize {
        stop.idx
    }

    fn nb_of_missions(&self) -> usize {
        self.timetables.nb_of_missions()
    }

    fn mission_id(&self, mission: &Self::Mission) -> usize {
        self.timetables.mission_id(mission)
    }
}

impl<Timetables: TimetablesTrait> data_interface::DataIO for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    fn new(
        model: &Model,
        loads_data: &LoadsData,
        default_transfer_duration: PositiveDuration,
        restrict_calendar: Option<(NaiveDate, NaiveDate)>,
    ) -> Self {
        Self::_new(
            model,
            loads_data,
            default_transfer_duration,
            restrict_calendar,
        )
    }
}

impl<'a, Timetables> data_interface::DataIters<'a> for TransitData<Timetables>
where
    Timetables: TimetablesTrait + TimetablesIter<'a>,
    Timetables::Mission: 'a,
    Timetables::Position: 'a,
{
    type MissionsAtStop = MissionsOfStop<'a, Timetables>;

    fn missions_at(&'a self, stop: &Self::Stop) -> Self::MissionsAtStop {
        self.missions_of(stop)
    }

    type OutgoingTransfersAtStop = iters::OutgoingTransfersAtStop<'a>;
    fn outgoing_transfers_at(&'a self, from_stop: &Self::Stop) -> Self::OutgoingTransfersAtStop {
        self.outgoing_transfers_at(from_stop)
    }

    type IncomingTransfersAtStop = iters::IncomingTransfersAtStop<'a>;
    fn incoming_transfers_at(&'a self, stop: &Self::Stop) -> Self::IncomingTransfersAtStop {
        self.incoming_transfers_at(stop)
    }

    type TripsOfMission = <Timetables as TimetablesIter<'a>>::Trips;

    fn trips_of(&'a self, mission: &Self::Mission) -> Self::TripsOfMission {
        self.timetables.trips_of(mission)
    }
}

impl<Timetables> data_interface::DataWithIters for TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
}

pub fn handle_insertion_errors(
    model: &ModelRefs,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    insertion_errors: &[InsertionError],
) {
    for error in insertion_errors {
        use crate::timetables::InsertionError::*;
        match error {
            Times(vehicle_journey_idx, error, dates) => {
                let _ = handle_vehicletimes_error(vehicle_journey_idx, dates, model, error);
            }
            VehicleJourneyAlreadyExistsOnDate(date, vehicle_journey_idx) => {
                let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
                error!(
                    "Trying to insert the vehicle journey {} more than once on day {}",
                    vehicle_journey_name, date
                );
            }
            InvalidDate(date, vehicle_journey_idx) => {
                let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);
                error!(
                    "Trying to insert the vehicle journey {} on day {},  \
                        but this day is not allowed in the date.  \
                        Allowed dates are between {} and {}",
                    vehicle_journey_name, date, start_date, end_date,
                );
            }
        }
    }
}

fn handle_vehicletimes_error(
    vehicle_journey_idx: &VehicleJourneyIdx,
    dates: &[NaiveDate],
    model: &ModelRefs<'_>,
    error: &VehicleTimesError,
) -> Result<(), ()> {
    if dates.is_empty() {
        error!("Received a vehicle times error with no date");
        return Err(());
    }

    let days_strings: Vec<String> = dates
        .iter()
        .map(|date| date.format("%H:%M:%S %d-%b-%y").to_string())
        .collect();

    let date = dates.first().unwrap();

    let vehicle_journey_name = model.vehicle_journey_name(vehicle_journey_idx);

    match error {
        VehicleTimesError::DebarkBeforeUpstreamBoard(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) =
                upstream_downstream_stop_names(model, vehicle_journey_idx, date, position_pair)?;
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    debark time at {}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingBoardTime(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) =
                upstream_downstream_stop_names(model, vehicle_journey_idx, date, position_pair)?;
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    board time at {}-th stop_time ({}) \
                    is earlier than its \
                    board time upstream at {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
        VehicleTimesError::DecreasingDebarkTime(position_pair) => {
            let (upstream_stop_name, downstream_stop_name) =
                upstream_downstream_stop_names(model, vehicle_journey_idx, date, position_pair)?;
            error!(
                "Skipping vehicle journey {} on days {:?} because its \
                    debark time at {}-th stop_time ({}) \
                    is earlier than its \
                    debark time upstream at {}-th stop_time ({}). ",
                vehicle_journey_name,
                days_strings,
                position_pair.downstream,
                downstream_stop_name,
                position_pair.upstream,
                upstream_stop_name
            );
        }
    }

    Ok(())
}

fn upstream_downstream_stop_names<'model>(
    model: &'model ModelRefs<'model>,
    vehicle_journey_idx: &VehicleJourneyIdx,
    date: &NaiveDate,
    position_pair: &PositionPair,
) -> Result<(&'model str, &'model str), ()> {
    let upstream_stop = model
        .stop_point_at(vehicle_journey_idx, position_pair.upstream, date)
        .ok_or_else(|| {
            error!(
                "Received a position pair with invalid upstream stop. \
                    Vehicle journey {} on {} upstream {}.",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                position_pair.upstream
            )
        })?;
    let upstream_stop_name = model.stop_point_name(&upstream_stop);

    let dowstream_stop = model
        .stop_point_at(vehicle_journey_idx, position_pair.downstream, date)
        .ok_or_else(|| {
            error!(
                "Received a position pair with invalid downstream stop. \
                    Vehicle journey {} on {} downstream {}.",
                model.vehicle_journey_name(vehicle_journey_idx),
                date,
                position_pair.downstream
            )
        })?;

    let downstream_stop_name = model.stop_point_name(&dowstream_stop);

    Ok((upstream_stop_name, downstream_stop_name))
}

pub fn handle_removal_errors(
    model: &ModelRefs,
    start_date: &NaiveDate,
    end_date: &NaiveDate,
    removal_errors: impl Iterator<Item = RemovalError>,
) {
    for error in removal_errors {
        match error {
            RemovalError::UnknownDate(date, vehicle_journey_idx) => {
                let vehicle_journey_name = model.vehicle_journey_name(&vehicle_journey_idx);
                error!(
                    "Trying to remove the vehicle journey {} on day {},  \
                        but this day is not allowed in the data.  \
                        Allowed dates are between {} and {}",
                    vehicle_journey_name, date, start_date, end_date,
                );
            }
            RemovalError::UnknownVehicleJourney(vehicle_journey_idx) => {
                let vehicle_journey_name = model.vehicle_journey_name(&vehicle_journey_idx);
                error!(
                    "Trying to remove the vehicle journey {},  \
                        but this vehicle journey is unknown",
                    vehicle_journey_name,
                );
            }
            RemovalError::DateInvalidForVehicleJourney(date, vehicle_journey_idx) => {
                let vehicle_journey_name = model.vehicle_journey_name(&vehicle_journey_idx);
                error!(
                    "Trying to remove the vehicle journey {} on day {},  \
                        but this vehicle journeys does not exists on this day. ",
                    vehicle_journey_name, date,
                );
            }
        }
    }
}
