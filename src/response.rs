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

use crate::{
    loads_data::LoadsCount,
    model::{ModelRefs, StopPointIdx, TransferIdx, VehicleJourneyIdx},
    time::{PositiveDuration, SecondsSinceDatasetUTCStart},
};
use chrono::{NaiveDate, NaiveDateTime};

use crate::transit_data::data_interface::Data as DataTrait;

use std::fmt::Debug;

use crate::request::generic_request::{MaximizeDepartureTimeError, MinimizeArrivalTimeError};

pub use typed_index_collection::Idx;

pub struct Response {
    pub departure: DepartureSection,
    pub first_vehicle: VehicleSection,
    pub connections: Vec<(TransferSection, WaitingSection, VehicleSection)>,
    pub arrival: ArrivalSection,
    pub loads_count: LoadsCount,
}

pub struct VehicleSection {
    pub from_datetime: NaiveDateTime,
    pub to_datetime: NaiveDateTime,
    pub vehicle_journey: VehicleJourneyIdx,
    pub day_for_vehicle_journey: NaiveDate,
    // the index (in vehicle_journey.stop_times) of the stop_time we board at
    pub from_stoptime_idx: usize,
    // the index (in vehicle_journey.stop_times) of the stop_time we debark at
    pub to_stoptime_idx: usize,
}

pub struct TransferSection {
    pub transfer: TransferIdx,
    pub from_datetime: NaiveDateTime,
    pub to_datetime: NaiveDateTime,
    pub from_stop_point: StopPointIdx,
    pub to_stop_point: StopPointIdx,
}

pub struct WaitingSection {
    pub from_datetime: NaiveDateTime,
    pub to_datetime: NaiveDateTime,
    pub stop_point: StopPointIdx,
}

pub struct DepartureSection {
    pub from_datetime: NaiveDateTime,
    pub to_datetime: NaiveDateTime,
    pub to_stop_point: StopPointIdx,
}

pub struct ArrivalSection {
    pub from_datetime: NaiveDateTime,
    pub to_datetime: NaiveDateTime,
    pub from_stop_point: StopPointIdx,
}

impl Response {
    pub fn first_vj_uri<'model>(&self, model: &'model ModelRefs<'model>) -> &'model str {
        let idx = &self.first_vehicle.vehicle_journey;
        model.vehicle_journey_name(idx)
    }
}

pub struct VehicleLeg<Data: DataTrait> {
    pub trip: Data::Trip,
    pub board_position: Data::Position,
    pub debark_position: Data::Position,
}

impl<Data: DataTrait> Clone for VehicleLeg<Data> {
    fn clone(&self) -> Self {
        Self {
            trip: self.trip.clone(),
            board_position: self.board_position.clone(),
            debark_position: self.debark_position.clone(),
        }
    }
}

#[derive(Clone)]
pub struct Journey<Data: DataTrait> {
    pub(crate) departure_datetime: SecondsSinceDatasetUTCStart,
    pub(crate) departure_fallback_duration: PositiveDuration,
    pub(crate) first_vehicle: VehicleLeg<Data>,
    pub(crate) connections: Vec<(Data::Transfer, VehicleLeg<Data>)>,
    pub(crate) arrival_fallback_duration: PositiveDuration,
    pub(crate) loads_count: LoadsCount,
}
#[derive(Debug, Clone)]
pub enum VehicleLegIdx {
    First,
    Connection(usize),
}

#[derive(Clone)]
pub enum JourneyError<Data: DataTrait> {
    BadJourney(BadJourney<Data>),
    MinimizeArrivalTimeError(MinimizeArrivalTimeError<Data>),
    MaximizeDepartureTimeError(MaximizeDepartureTimeError<Data>),
}

impl<Data: DataTrait> Debug for JourneyError<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JourneyError::BadJourney(err) => {
                write!(f, "BadJourney : {:?}", err)
            }
            JourneyError::MinimizeArrivalTimeError(err) => {
                write!(f, "MinimizeArrivalTimeError : {:?}", err)
            }
            JourneyError::MaximizeDepartureTimeError(err) => {
                write!(f, "MaximizeDepartureTimeError : {:?}", err)
            }
        }
    }
}

#[derive(Clone)]
pub enum BadJourney<Data: DataTrait> {
    DebarkIsUpstreamBoard(VehicleLeg<Data>, VehicleLegIdx),
    NoBoardTime(VehicleLeg<Data>, VehicleLegIdx),
    NoDebarkTime(VehicleLeg<Data>, VehicleLegIdx),
    BadTransferStartStop(VehicleLeg<Data>, Data::Transfer, usize),
    BadTransferEndStop(Data::Transfer, VehicleLeg<Data>, usize),
    BadTransferEndTime(Data::Transfer, VehicleLeg<Data>, usize),
}

impl<Data: DataTrait> Debug for BadJourney<Data> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadJourney::DebarkIsUpstreamBoard(_, _) => {
                write!(f, "DebarkIsUpstreamBoard")
            }
            BadJourney::NoBoardTime(_, _) => {
                write!(f, "NoBoardTime")
            }
            BadJourney::NoDebarkTime(_, _) => {
                write!(f, "NoDebarkTime")
            }
            BadJourney::BadTransferStartStop(_, _, _) => {
                write!(f, "BadTransferStartStop")
            }
            BadJourney::BadTransferEndStop(_, _, _) => {
                write!(f, "BadTransferEndStop")
            }
            BadJourney::BadTransferEndTime(_, _, _) => {
                write!(f, "BadTransferEndTime")
            }
        }
    }
}

impl<Data> Journey<Data>
where
    Data: DataTrait,
{
    pub fn new(
        departure_datetime: SecondsSinceDatasetUTCStart,
        departure_fallback_duration: PositiveDuration,
        first_vehicle: VehicleLeg<Data>,
        connections: impl Iterator<Item = (Data::Transfer, VehicleLeg<Data>)>,
        arrival_fallback_duration: PositiveDuration,
        loads_count: LoadsCount,
        data: &Data,
    ) -> Result<Self, BadJourney<Data>> {
        let result = Self {
            departure_datetime,
            departure_fallback_duration,
            first_vehicle,
            arrival_fallback_duration,
            connections: connections.collect(),
            loads_count,
        };

        result.is_valid(data)?;

        Ok(result)
    }

    fn is_valid(&self, data: &Data) -> Result<(), BadJourney<Data>> {
        let (first_debark_stop, first_debark_time) = {
            let board_position = &self.first_vehicle.board_position;
            let debark_position = &self.first_vehicle.debark_position;
            let trip = &self.first_vehicle.trip;
            let mission = data.mission_of(trip);
            if data.is_upstream(debark_position, board_position, &mission) {
                return Err(BadJourney::DebarkIsUpstreamBoard(
                    self.first_vehicle.clone(),
                    VehicleLegIdx::First,
                ));
            }

            if data.board_time_of(trip, board_position).is_none() {
                return Err(BadJourney::NoBoardTime(
                    self.first_vehicle.clone(),
                    VehicleLegIdx::First,
                ));
            }

            let debark_timeload = data.debark_time_of(trip, debark_position).ok_or_else(|| {
                BadJourney::NoDebarkTime(self.first_vehicle.clone(), VehicleLegIdx::First)
            })?;

            let debark_stop = data.stop_of(debark_position, &mission);
            (debark_stop, debark_timeload.0)
        };

        let mut prev_debark_stop = first_debark_stop;
        let mut prev_debark_time = first_debark_time;
        let mut prev_vehicle_leg = &self.first_vehicle;

        for (idx, (transfer, vehicle_leg)) in self.connections.iter().enumerate() {
            let (transfer_from_stop, transfer_to_stop) = data.transfer_from_to_stop(transfer);
            let transfer_duration = data.transfer_duration(transfer);

            if !data.is_same_stop(&prev_debark_stop, &transfer_from_stop) {
                return Err(BadJourney::BadTransferStartStop(
                    prev_vehicle_leg.clone(),
                    transfer.clone(),
                    idx,
                ));
            }

            let board_position = &vehicle_leg.board_position;
            let debark_position = &vehicle_leg.debark_position;
            let trip = &vehicle_leg.trip;
            let mission = data.mission_of(trip);
            if data.is_upstream(debark_position, board_position, &mission) {
                return Err(BadJourney::DebarkIsUpstreamBoard(
                    vehicle_leg.clone(),
                    VehicleLegIdx::Connection(idx),
                ));
            }

            let board_timeload = data.board_time_of(trip, board_position).ok_or_else(|| {
                BadJourney::NoBoardTime(vehicle_leg.clone(), VehicleLegIdx::Connection(idx))
            })?;
            let board_time = board_timeload.0;

            let debark_timeload = data.debark_time_of(trip, debark_position).ok_or_else(|| {
                BadJourney::NoDebarkTime(vehicle_leg.clone(), VehicleLegIdx::Connection(idx))
            })?;
            let debark_time = debark_timeload.0;

            let board_stop = data.stop_of(board_position, &mission);
            let debark_stop = data.stop_of(debark_position, &mission);

            if !data.is_same_stop(&transfer_to_stop, &board_stop) {
                return Err(BadJourney::BadTransferEndStop(
                    transfer.clone(),
                    vehicle_leg.clone(),
                    idx,
                ));
            }

            let end_transfer_time = prev_debark_time + transfer_duration;
            if end_transfer_time > board_time {
                return Err(BadJourney::BadTransferEndTime(
                    transfer.clone(),
                    vehicle_leg.clone(),
                    idx,
                ));
            }

            prev_debark_time = debark_time;
            prev_debark_stop = debark_stop;
            prev_vehicle_leg = vehicle_leg;
        }

        Ok(())
    }

    pub fn first_vehicle_board_datetime(&self, data: &Data) -> NaiveDateTime {
        let seconds = self.first_vehicle_board_time(data);
        data.to_naive_datetime(&seconds)
    }

    fn first_vehicle_board_time(&self, data: &Data) -> SecondsSinceDatasetUTCStart {
        data.board_time_of(&self.first_vehicle.trip, &self.first_vehicle.board_position)
            .unwrap()
            .0
    }

    fn last_vehicle_leg(&self) -> &VehicleLeg<Data> {
        self.connections
            .last()
            .map(|(_, vehicle_leg)| vehicle_leg)
            .unwrap_or(&self.first_vehicle)
    }

    fn last_vehicle_debark_time(&self, data: &Data) -> SecondsSinceDatasetUTCStart {
        let last_vehicle_leg = self.last_vehicle_leg();
        data.debark_time_of(&last_vehicle_leg.trip, &last_vehicle_leg.debark_position)
            .unwrap()
            .0 //unwrap is safe because of checks that happens during Self construction
    }

    pub fn last_vehicle_debark_datetime(&self, data: &Data) -> NaiveDateTime {
        let seconds = self.last_vehicle_debark_time(data);
        data.to_naive_datetime(&seconds)
    }

    fn arrival(&self, data: &Data) -> SecondsSinceDatasetUTCStart {
        let last_debark_time = self.last_vehicle_debark_time(data);
        last_debark_time + self.arrival_fallback_duration
    }

    pub fn total_transfer_duration(&self, data: &Data) -> PositiveDuration {
        let mut result = PositiveDuration::zero();
        for (transfer, _) in &self.connections {
            let transfer_duration = data.transfer_duration(transfer);
            result = result + transfer_duration;
        }
        result
    }

    pub fn total_duration(&self, data: &Data) -> PositiveDuration {
        let arrival_time = self.arrival(data);
        let departure_time = self.departure_datetime;
        //unwrap is safe because of checks that happens during Self construction
        arrival_time.duration_since(&departure_time).unwrap()
    }

    pub fn total_duration_in_pt(&self, data: &Data) -> PositiveDuration {
        let arrival_time = self.last_vehicle_debark_time(data);
        let departure_time = self.first_vehicle_board_time(data);
        //unwrap is safe because of checks that happens during Self construction
        arrival_time.duration_since(&departure_time).unwrap()
    }

    pub fn nb_of_legs(&self) -> usize {
        self.connections.len() + 1
    }

    pub fn nb_of_connections(&self) -> usize {
        self.connections.len()
    }

    pub fn nb_of_transfers(&self) -> usize {
        self.connections.len()
    }

    pub fn departure_datetime(&self, data: &Data) -> NaiveDateTime {
        data.to_naive_datetime(&self.departure_datetime)
    }

    pub fn arrival_datetime(&self, data: &Data) -> NaiveDateTime {
        let arrival_time = self.arrival(data);
        data.to_naive_datetime(&arrival_time)
    }

    pub fn total_fallback_duration(&self) -> PositiveDuration {
        self.departure_fallback_duration + self.arrival_fallback_duration
    }

    pub fn print(&self, data: &Data, model: &ModelRefs<'_>) -> Result<String, std::fmt::Error> {
        let mut result = String::new();
        self.write(data, model, &mut result)?;
        Ok(result)
    }

    fn write_date(date: &NaiveDateTime) -> String {
        date.format("%H:%M:%S %d-%b-%y").to_string()
    }

    pub fn write<Writer: std::fmt::Write>(
        &self,
        data: &Data,
        model: &ModelRefs<'_>,
        writer: &mut Writer,
    ) -> Result<(), std::fmt::Error> {
        writeln!(writer, "*** New journey ***")?;
        let arrival_time = self.arrival(data);
        let arrival_datetime = data.to_naive_datetime(&arrival_time);
        writeln!(writer, "Arrival : {}", Self::write_date(&arrival_datetime))?;
        writeln!(
            writer,
            "Transfer duration : {}",
            self.total_transfer_duration(data)
        )?;
        writeln!(writer, "Nb of vehicles : {}", self.nb_of_legs())?;
        writeln!(
            writer,
            "Fallback  total: {}, start {}, end {}",
            self.total_fallback_duration(),
            self.departure_fallback_duration,
            self.arrival_fallback_duration
        )?;
        writeln!(writer, "Loads : {}", self.loads_count)?;

        let departure_datetime = data.to_naive_datetime(&self.departure_datetime);
        writeln!(
            writer,
            "Departure : {}",
            Self::write_date(&departure_datetime)
        )?;

        self.write_vehicle_leg(&self.first_vehicle, data, model, writer)?;
        for (_, vehicle_leg) in self.connections.iter() {
            self.write_vehicle_leg(vehicle_leg, data, model, writer)?;
        }

        Ok(())
    }

    fn write_vehicle_leg<Writer: std::fmt::Write>(
        &self,
        vehicle_leg: &VehicleLeg<Data>,
        data: &Data,
        model: &ModelRefs<'_>,
        writer: &mut Writer,
    ) -> Result<(), std::fmt::Error> {
        let trip = &vehicle_leg.trip;
        let vehicle_journey_idx = data.vehicle_journey_idx(trip);

        let line_id = model.line_name(&vehicle_journey_idx);

        let mission = data.mission_of(trip);

        let from_stop = data.stop_of(&vehicle_leg.board_position, &mission);
        let to_stop = data.stop_of(&vehicle_leg.debark_position, &mission);
        let from_stop_idx = data.stop_point_idx(&from_stop);
        let to_stop_idx = data.stop_point_idx(&to_stop);
        let from_stop_id = model.stop_point_name(&from_stop_idx);
        let to_stop_id = model.stop_point_name(&to_stop_idx);

        let board_time = data
            .board_time_of(trip, &vehicle_leg.board_position)
            .unwrap()
            .0;
        let board_datetime = data.to_naive_datetime(&board_time);
        let debark_time = data
            .debark_time_of(trip, &vehicle_leg.debark_position)
            .unwrap()
            .0;
        let debark_datetime = data.to_naive_datetime(&debark_time);

        let from_datetime = Self::write_date(&board_datetime);
        let to_datetime = Self::write_date(&debark_datetime);
        writeln!(
            writer,
            "{} from {} at {} to {} at {} ",
            line_id, from_stop_id, from_datetime, to_stop_id, to_datetime
        )?;
        Ok(())
    }
}

impl<Data: DataTrait> Journey<Data> {
    pub fn to_response(&self, data: &Data) -> Response {
        Response {
            departure: self.departure_section(data),
            first_vehicle: self.first_vehicle_section(data),
            connections: self.connections(data).collect(),
            arrival: self.arrival_section(data),
            loads_count: self.loads_count.clone(),
        }
    }

    pub fn departure_section(&self, data: &Data) -> DepartureSection {
        let from_datetime = data.to_naive_datetime(&self.departure_datetime);
        let to_seconds = self.departure_datetime + self.departure_fallback_duration;
        let to_datetime = data.to_naive_datetime(&to_seconds);
        let position = self.first_vehicle.debark_position.clone();
        let trip = &self.first_vehicle.trip;
        let mission = data.mission_of(trip);
        let stop = data.stop_of(&position, &mission);
        let to_stop_point = data.stop_point_idx(&stop);
        DepartureSection {
            from_datetime,
            to_datetime,
            to_stop_point,
        }
    }

    pub fn arrival_section(&self, data: &Data) -> ArrivalSection {
        let from_time = self.last_vehicle_debark_time(data);
        let to_time = from_time + self.arrival_fallback_duration;
        let last_vehicle_leg = self.last_vehicle_leg();
        let position = &last_vehicle_leg.debark_position;
        let trip = &last_vehicle_leg.trip;
        let mission = data.mission_of(trip);
        let stop = data.stop_of(position, &mission);
        let stop_point = data.stop_point_idx(&stop);
        ArrivalSection {
            from_datetime: data.to_naive_datetime(&from_time),
            to_datetime: data.to_naive_datetime(&to_time),
            from_stop_point: stop_point,
        }
    }

    pub fn first_vehicle_section(&self, data: &Data) -> VehicleSection {
        self.vehicle_section(&VehicleLegIdx::First, data)
    }

    fn vehicle_section(&self, vehicle_leg_idx: &VehicleLegIdx, data: &Data) -> VehicleSection {
        let vehicle_leg = match vehicle_leg_idx {
            VehicleLegIdx::First => &self.first_vehicle,
            VehicleLegIdx::Connection(idx) => &self.connections[*idx].1,
        };
        let trip = &vehicle_leg.trip;
        let vehicle_journey = data.vehicle_journey_idx(trip);

        let from_stoptime_idx = data.stoptime_idx(&vehicle_leg.board_position, trip);
        let to_stoptime_idx = data.stoptime_idx(&vehicle_leg.debark_position, trip);

        //unwraps below are safe because of checks that happens during Self::new()
        let board_time = data
            .board_time_of(trip, &vehicle_leg.board_position)
            .unwrap()
            .0;
        let debark_time = data
            .debark_time_of(trip, &vehicle_leg.debark_position)
            .unwrap()
            .0;

        let from_datetime = data.to_naive_datetime(&board_time);
        let to_datetime = data.to_naive_datetime(&debark_time);

        let day_for_vehicle_journey = data.day_of(trip);

        VehicleSection {
            from_datetime,
            to_datetime,
            from_stoptime_idx,
            to_stoptime_idx,
            vehicle_journey,
            day_for_vehicle_journey,
        }
    }

    fn transfer_section(&self, connection_idx: usize, data: &Data) -> TransferSection {
        let prev_vehicle_leg = if connection_idx == 0 {
            &self.first_vehicle
        } else {
            &self.connections[connection_idx - 1].1
        };
        let prev_trip = &prev_vehicle_leg.trip;
        let prev_debark_time = data
            .debark_time_of(prev_trip, &prev_vehicle_leg.debark_position)
            .unwrap()
            .0;
        let from_datetime = data.to_naive_datetime(&prev_debark_time);

        let (transfer, _) = &self.connections[connection_idx];
        let (transfer_from_stop, transfer_to_stop) = data.transfer_from_to_stop(transfer);
        let transfer_duration = data.transfer_duration(transfer);
        let end_transfer_time = prev_debark_time + transfer_duration;
        let to_datetime = data.to_naive_datetime(&end_transfer_time);
        let to_stop_point = data.stop_point_idx(&transfer_to_stop);
        let from_stop_point = data.stop_point_idx(&transfer_from_stop);
        let transfer_idx = data.transfer_idx(transfer);
        TransferSection {
            transfer: transfer_idx,
            from_datetime,
            to_datetime,
            from_stop_point,
            to_stop_point,
        }
    }

    pub fn connections<'journey, 'data>(
        &'journey self,
        data: &'data Data,
    ) -> ConnectionIter<'journey, 'data, Data> {
        ConnectionIter {
            data,
            journey: self,
            connection_idx: 0,
        }
    }
}

pub struct ConnectionIter<'journey, 'data, Data: DataTrait> {
    data: &'data Data,
    journey: &'journey Journey<Data>,
    connection_idx: usize,
}

impl<'journey, 'data, Data: DataTrait> Iterator for ConnectionIter<'journey, 'data, Data> {
    type Item = (TransferSection, WaitingSection, VehicleSection);

    fn next(&mut self) -> Option<Self::Item> {
        if self.connection_idx >= self.journey.connections.len() {
            return None;
        }

        let transfer_section = self
            .journey
            .transfer_section(self.connection_idx, self.data);
        let vehicle_section = self
            .journey
            .vehicle_section(&VehicleLegIdx::Connection(self.connection_idx), self.data);
        let waiting_section = WaitingSection {
            from_datetime: transfer_section.to_datetime,
            to_datetime: vehicle_section.from_datetime,
            stop_point: transfer_section.to_stop_point.clone(),
        };
        self.connection_idx += 1;
        Some((transfer_section, waiting_section, vehicle_section))
    }
}

impl VehicleSection {
    fn duration_in_seconds(&self) -> i64 {
        let duration = self.to_datetime - self.from_datetime;
        duration.num_seconds()
    }

    pub fn from_stop_point_name<'a>(&self, model: &'a ModelRefs<'a>) -> Option<&'a str> {
        model
            .stop_point_at(
                &self.vehicle_journey,
                self.from_stoptime_idx,
                &self.day_for_vehicle_journey,
            )
            .map(|idx| model.stop_point_name(&idx))
    }

    pub fn to_stop_point_name<'a>(&self, model: &'a ModelRefs<'a>) -> Option<&'a str> {
        model
            .stop_point_at(
                &self.vehicle_journey,
                self.to_stoptime_idx,
                &self.day_for_vehicle_journey,
            )
            .map(|idx| model.stop_point_name(&idx))
    }

    fn write<Writer: std::fmt::Write>(
        &self,
        model: &ModelRefs<'_>,
        writer: &mut Writer,
    ) -> Result<(), std::fmt::Error> {
        let vehicle_journey_idx = &self.vehicle_journey;
        // let route_id = real_time_model.route_name(&vehicle_journey_idx, model);
        let line_id = model.line_name(vehicle_journey_idx);

        let from_stop_id = model
            .stop_point_at(
                vehicle_journey_idx,
                self.from_stoptime_idx,
                &self.day_for_vehicle_journey,
            )
            .map(|stop_idx| model.stop_point_name(&stop_idx))
            .unwrap_or("unknown_stop");
        let to_stop_id = model
            .stop_point_at(
                vehicle_journey_idx,
                self.to_stoptime_idx,
                &self.day_for_vehicle_journey,
            )
            .map(|stop_idx| model.stop_point_name(&stop_idx))
            .unwrap_or("unknown_stop");

        let from_datetime = write_date(&self.from_datetime);
        let to_datetime = write_date(&self.to_datetime);
        writeln!(
            writer,
            "{} from {} at {} to {} at {} ",
            line_id, from_stop_id, from_datetime, to_stop_id, to_datetime
        )?;
        Ok(())
    }
}

impl TransferSection {
    fn duration_in_seconds(&self) -> i64 {
        let duration = self.to_datetime - self.from_datetime;
        duration.num_seconds()
    }
}

impl ArrivalSection {
    fn duration_in_seconds(&self) -> i64 {
        let duration = self.to_datetime - self.from_datetime;
        duration.num_seconds()
    }
}

impl DepartureSection {
    fn duration_in_seconds(&self) -> i64 {
        let duration = self.to_datetime - self.from_datetime;
        duration.num_seconds()
    }
}

impl Response {
    pub fn nb_of_sections(&self) -> usize {
        1 + 3 * self.connections.len()
    }

    /// number of seconds spent in public transport
    pub fn total_duration_in_pt(&self) -> i64 {
        let first_vehicle_duration = self.first_vehicle.duration_in_seconds();
        let remaining_duration: i64 = self
            .connections
            .iter()
            .map(|(_, _, vehicle_section)| vehicle_section.duration_in_seconds())
            .sum();
        first_vehicle_duration + remaining_duration
    }

    pub fn nb_of_transfers(&self) -> usize {
        self.connections.len()
    }

    pub fn first_vehicle_board_datetime(&self) -> NaiveDateTime {
        self.first_vehicle.from_datetime
    }

    pub fn last_vehicle_debark_datetime(&self) -> NaiveDateTime {
        let last_vehicle_section = self
            .connections
            .last()
            .map(|(_, _, vehicle_section)| vehicle_section)
            .unwrap_or(&self.first_vehicle);
        last_vehicle_section.to_datetime
    }

    pub fn total_transfer_duration(&self) -> i64 {
        self.connections
            .iter()
            .map(|(transfer_section, _, _)| transfer_section.duration_in_seconds())
            .sum()
    }

    pub fn total_fallback_duration(&self) -> i64 {
        self.departure.duration_in_seconds() + self.arrival.duration_in_seconds()
    }

    pub fn total_walking_duration(&self) -> i64 {
        self.total_fallback_duration() + self.total_transfer_duration()
    }

    pub fn total_duration(&self) -> i64 {
        let duration = self.arrival.to_datetime - self.departure.from_datetime;
        duration.num_seconds()
    }

    pub fn nb_of_vehicles(&self) -> usize {
        self.connections.len() + 1
    }

    pub fn print(&self, model: &ModelRefs<'_>) -> Result<String, std::fmt::Error> {
        let mut result = String::new();
        self.write(model, &mut result)?;
        Ok(result)
    }

    pub fn write<Writer: std::fmt::Write>(
        &self,
        model: &ModelRefs<'_>,
        writer: &mut Writer,
    ) -> Result<(), std::fmt::Error> {
        writeln!(writer, "*** New journey ***")?;
        let arrival_datetime = self.arrival.to_datetime;
        writeln!(writer, "Arrival : {}", write_date(&arrival_datetime))?;
        writeln!(
            writer,
            "Transfer duration : {}",
            write_duration(self.total_transfer_duration())
        )?;
        writeln!(writer, "Nb of vehicles : {}", self.nb_of_vehicles())?;
        writeln!(
            writer,
            "Fallback total: {}, start {}, end {}",
            write_duration(self.total_fallback_duration()),
            write_duration(self.departure.duration_in_seconds()),
            write_duration(self.arrival.duration_in_seconds())
        )?;
        writeln!(writer, "Loads : {}", self.loads_count)?;

        writeln!(
            writer,
            "Departure : {}",
            write_date(&self.departure.from_datetime)
        )?;

        self.first_vehicle.write(model, writer)?;
        for (_, _, vehicle) in self.connections.iter() {
            vehicle.write(model, writer)?;
        }

        Ok(())
    }
}

fn write_date(date: &NaiveDateTime) -> String {
    date.format("%H:%M:%S %d-%b-%y").to_string()
}

fn write_duration(seconds: i64) -> String {
    let hours = seconds / (60 * 60);
    let minutes_in_secs = seconds % (60 * 60);
    let minutes = minutes_in_secs / 60;
    let seconds = minutes_in_secs % 60;
    if hours != 0 {
        format!("{}h{:02}m{:02}s", hours, minutes, seconds)
    } else if minutes != 0 {
        format!("{}m{:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
