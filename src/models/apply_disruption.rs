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
    chrono::NaiveDate,
    time::calendar,
    transit_data::{
        data_interface::Data as DataTrait, handle_insertion_error, handle_modify_error,
        handle_removal_error,
    },
};

use tracing::{debug, error, trace, warn};

use super::{
    real_time_disruption::{
        TimePeriods,  VehicleJourneyId,
    },

    RealTimeModel,
};
use crate::models::real_time_disruption::intersection;
use crate::models::{StopPointIdx, VehicleJourneyIdx};
use crate::{

    DataUpdate,
};

use super::{base_model::BaseModel, real_time_disruption as disruption, ModelRefs};

pub enum ActionType {
    ApplyImpact,
    ApplyInform,
    CancelImpact,
    CancelInform,
}

impl RealTimeModel {
    pub fn cancel_disruption_by_id<Data: DataTrait + DataUpdate>(
        &mut self,
        disruption_id: &str,
        base_model: &BaseModel,
        data: &mut Data,
    ) {
        let disruption_idx = self
            .disruptions
            .iter()
            .position(|disruption| disruption.id == disruption_id);

        if let Some(idx) = disruption_idx {
            let disruption = self.disruptions[idx].clone();
            let disruption_idx = DisruptionIdx { idx };
            for (idx, impact) in disruption.impacts.iter().enumerate() {
                let impact_idx = ImpactIdx { idx };
                self.cancel_impact(impact, base_model, data, &disruption_idx, &impact_idx);
            }
        } else {
            warn!("Cannot cancel disruption with id {}, as it was not found in realtime_model.disruptions",
                   disruption_id)
        }
    }

    fn cancel_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &disruption::Impact,
        base_model: &BaseModel,
        data: &mut Data,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) {
        let model_period = [base_model.time_period()];
        // filter application_periods by model_period
        // by taking the intersection of theses two TimePeriodsapplication_periods
        let application_periods: Vec<_> = impact
            .application_periods
            .iter()
            .filter_map(|application_periods| intersection(application_periods, &model_period[0]))
            .collect();

        if application_periods.is_empty() {
            return;
        }
        // unwrap is sfe here because we checked if application_periods is empty or not
        let application_periods = TimePeriods::new(&application_periods).unwrap();

        for pt_object in &impact.impacted_pt_objects {
            let result = match pt_object {
                Impacted::NetworkDeleted(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::LineDeleted(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::RouteDeleted(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::BaseTripDeleted(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::StopAreaDeleted(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::StopPointDeleted(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelImpact,
                ),
                Impacted::RailSection(_) => todo!(),
                Impacted::LineSection(_) => todo!(),
                // Kirin
                Impacted::TripDeleted(vehicle_journey_id, date) => self.delete_trip(
                    base_model,
                    data,
                    &vehicle_journey_id.id,
                    date,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::BaseTripUpdated(trip_disruption) => self.update_base_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::NewTripUpdated(trip_disruption) => self.update_new_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
            };
            if let Err(err) = result {
                error!("Error while applying impact {} : {:?}", impact.id, err);
            }
        }

        for pt_object in &impact.informed_pt_objects {
            let result = match pt_object {
                Informed::Network(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::Route(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::Line(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::Trip(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::StopArea(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::StopPoint(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::CancelInform,
                ),
                Informed::Unknown => todo!(),
            };
            if let Err(err) = result {
                error!(
                    "Error while storing informed impact {} : {:?}",
                    impact.id, err
                );
            }
        }
    }

    pub fn store_and_apply_disruption<Data: DataTrait + DataUpdate>(
        &mut self,
        disruption: disruption::Disruption,
        base_model: &BaseModel,
        data: &mut Data,
    ) {
        let disruption_idx = DisruptionIdx {
            idx: self.disruptions.len(),
        };

        for (idx, impact) in disruption.impacts.iter().enumerate() {
            let impact_idx = ImpactIdx { idx };
            self.apply_impact(impact, base_model, data, &disruption_idx, &impact_idx);
        }

        self.disruptions.push(disruption);
    }

    fn apply_impact<Data: DataTrait + DataUpdate>(
        &mut self,
        impact: &disruption::Impact,
        base_model: &BaseModel,
        data: &mut Data,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) {
        let model_period = [base_model.time_period()];
        // filter application_periods by model_period
        // by taking the intersection of theses two TimePeriodsapplication_periods
        let application_periods: Vec<_> = impact
            .application_periods
            .iter()
            .filter_map(|application_periods| intersection(application_periods, &model_period[0]))
            .collect();

        if application_periods.is_empty() {
            return;
        }
        // unwrap is sfe here because we checked if application_periods is empty or not
        let application_periods = TimePeriods::new(&application_periods).unwrap();

        for pt_object in &impact.impacted_pt_objects {
            let result = match pt_object {
                Impacted::NetworkDeleted(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::LineDeleted(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::RouteDeleted(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::BaseTripDeleted(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::StopAreaDeleted(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::StopPointDeleted(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyImpact,
                ),
                Impacted::RailSection(_) => todo!(),
                Impacted::LineSection(_) => todo!(),
                // Kirin
                Impacted::TripDeleted(vehicle_journey_id, date) => self.delete_trip(
                    base_model,
                    data,
                    &vehicle_journey_id.id,
                    date,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::BaseTripUpdated(trip_disruption) => self.update_base_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
                Impacted::NewTripUpdated(trip_disruption) => self.update_new_trip(
                    base_model,
                    data,
                    trip_disruption,
                    disruption_idx,
                    impact_idx,
                ),
            };
            if let Err(err) = result {
                error!("Error while applying impact {} : {:?}", impact.id, err);
            }
        }

        for pt_object in &impact.informed_pt_objects {
            let result = match pt_object {
                Informed::Network(network) => self.apply_on_network(
                    base_model,
                    data,
                    &network.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::Route(route) => self.apply_on_route(
                    base_model,
                    data,
                    &route.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::Line(line) => self.apply_on_line(
                    base_model,
                    data,
                    &line.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::Trip(trip) => self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    &trip.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::StopArea(stop_area) => self.apply_on_stop_area(
                    base_model,
                    data,
                    &stop_area.id,
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::StopPoint(stop_point) => self.apply_on_stop_point(
                    base_model,
                    data,
                    &[&stop_point.id],
                    &application_periods,
                    disruption_idx,
                    impact_idx,
                    &ActionType::ApplyInform,
                ),
                Informed::Unknown => todo!(),
            };
            if let Err(err) = result {
                error!(
                    "Error while storing informed impact {} : {:?}",
                    impact.id, err
                );
            }
        }
    }

    //----------------------------------------------------------------------------------------
    // functions operating on TC objects for KIRIN
    fn update_new_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;

        let has_base_vj_idx = base_model.vehicle_journey_idx(vehicle_journey_id);
        let date = trip_disruption.trip_date;
        let trip_exists_in_base = {
            match has_base_vj_idx {
                None => false,
                Some(vj_idx) => base_model.trip_exists(vj_idx, date),
            }
        };

        if trip_exists_in_base {
            return Err(DisruptionError::NewTripWithBaseId(
                VehicleJourneyId {
                    id: vehicle_journey_id.to_string(),
                },
                date,
            ));
        }
        let stop_times = self.make_stop_times(&trip_disruption.stop_times, base_model);

        if self.is_present(vehicle_journey_id, &date, base_model) {
            self.modify_trip(
                base_model,
                data,
                vehicle_journey_id,
                &date,
                stop_times,
                disruption_idx,
                impact_idx,
            )
        } else {
            self.add_trip(base_model, data, vehicle_journey_id, &date, stop_times)
        }
    }

    fn update_base_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        trip_disruption: &TripDisruption,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        let vehicle_journey_id = &trip_disruption.trip_id.id;
        let stop_times = self.make_stop_times(&trip_disruption.stop_times, base_model);

        if let Some(base_vj_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            let date = trip_disruption.trip_date;
            let trip_exists_in_base = base_model.trip_exists(base_vj_idx, date);

            if !trip_exists_in_base {
                return Err(DisruptionError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    date,
                ));
            }

            if self.is_present(vehicle_journey_id, &date, base_model) {
                self.modify_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    &date,
                    stop_times,
                    disruption_idx,
                    impact_idx,
                )
            } else {
                self.add_trip(base_model, data, vehicle_journey_id, &date, stop_times)
            }
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.clone(),
            }))
        }
    }

    //----------------------------------------------------------------------------------------
    // elementary functions operating on trips (VJ + date)
    // Used for chaos and kirin
    fn add_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Adding a new vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .add(vehicle_journey_id, date, stop_times, base_model)
            .map_err(|_| {
                DisruptionError::AddPresentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        trace!(
            "New vehicle journey {} on date {} stored in real time model. Stop times : {:#?} ",
            vehicle_journey_id,
            date,
            stop_times
        );
        let dates = std::iter::once(*date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);
        let insert_result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            &chrono_tz::UTC,
            vj_idx,
        );
        let model_ref = ModelRefs {
            base: base_model,
            real_time: self,
        };
        if let Err(err) = insert_result {
            handle_insertion_error(
                &model_ref,
                data.calendar().first_date(),
                data.calendar().last_date(),
                &err,
            );
        }

        Ok(())
    }

    pub fn modify_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Modifying vehicle journey {} on date {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .modify(vehicle_journey_id, date, stop_times, base_model)
            .map_err(|_| {
                DisruptionError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        let dates = std::iter::once(*date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

        let modify_result = data.modify_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            &chrono_tz::UTC,
            &vj_idx,
        );
        match modify_result {
            Ok(_) => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(err) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_modify_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &err,
                );
            }
        }
        Ok(())
    }

    fn delete_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Deleting vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let vj_idx = self
            .delete(vehicle_journey_id, date, base_model)
            .map_err(|_| {
                DisruptionError::DeleteAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;
        let removal_result = data.remove_real_time_vehicle(&vj_idx, date);
        match removal_result {
            Ok(_) => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(removal_error) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_removal_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &removal_error,
                );
            }
        }
        Ok(())
    }

    fn restore_base_trip<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
    ) -> Result<(), DisruptionError> {
        debug!(
            "Restore vehicle journey {} on day {}",
            vehicle_journey_id, date
        );
        let (vj_idx, stop_times) = self
            .restore_base_vehicle_journey(vehicle_journey_id, date, base_model)
            .map_err(|_| {
                DisruptionError::ModifyAbsentTrip(
                    VehicleJourneyId {
                        id: vehicle_journey_id.to_string(),
                    },
                    *date,
                )
            })?;

        let dates = std::iter::once(*date);
        let stops = stop_times.iter().map(|stop_time| stop_time.stop.clone());
        let flows = stop_times.iter().map(|stop_time| stop_time.flow_direction);
        let board_times = stop_times.iter().map(|stop_time| stop_time.board_time);
        let debark_times = stop_times.iter().map(|stop_time| stop_time.debark_time);

        let result = data.insert_real_time_vehicle(
            stops,
            flows,
            board_times,
            debark_times,
            base_model.loads_data(),
            dates,
            &chrono_tz::UTC,
            VehicleJourneyIdx::Base(vj_idx),
        );
        match result {
            Ok(_) => self.cancel_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            Err(err) => {
                let model_ref = ModelRefs {
                    base: base_model,
                    real_time: self,
                };
                handle_insertion_error(
                    &model_ref,
                    data.calendar().first_date(),
                    data.calendar().last_date(),
                    &err,
                );
            }
        }
        Ok(())
    }

    //----------------------------------------------------------------------------------------
    // functions operating on TC objects for CHAOS
    fn dispatch_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) {
        match action_type {
            ActionType::ApplyImpact => {
                let result = self.delete_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    disruption_idx,
                    impact_idx,
                );
                // we should never get a DeleteAbsentTrip error
                // since we check in trip_time_period() that this trip exists
                if let Err(err) = result {
                    error!(
                        "Unexpected error while deleting a base vehicle journey {:?}",
                        err
                    );
                }
            }
            ActionType::ApplyInform => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            ActionType::CancelImpact => {
                let result = self.restore_base_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    disruption_idx,
                    impact_idx,
                );
                if let Err(err) = result {
                    error!(
                        "Unexpected error while restoring a base vehicle journey {:?}",
                        err
                    );
                }
            }
            ActionType::CancelInform => self.cancel_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
        }
    }

    fn apply_on_base_vehicle_journey<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        if let Some(vehicle_journey_idx) = base_model.vehicle_journey_idx(vehicle_journey_id) {
            for date in application_periods.dates_possibly_concerned() {
                if let Some(trip_period) = base_model.trip_time_period(vehicle_journey_idx, &date) {
                    if application_periods.intersects(&trip_period) {
                        self.dispatch_on_base_vehicle_journey(
                            base_model,
                            data,
                            vehicle_journey_id,
                            &date,
                            disruption_idx,
                            impact_idx,
                            action_type,
                        );
                    }
                }
            }
            Ok(())
        } else {
            Err(DisruptionError::VehicleJourneyAbsent(VehicleJourneyId {
                id: vehicle_journey_id.to_string(),
            }))
        }
    }

    fn apply_on_network<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        network_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_network_id(network_id) {
            return Err(DisruptionError::NetworkAbsent(NetworkId {
                id: network_id.to_string(),
            }));
        }

        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.network_name(base_vehicle_journey_idx) == Some(network_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                    action_type,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_line<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        line_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_line_id(line_id) {
            return Err(DisruptionError::LineAbsent(LineId {
                id: line_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.line_name(base_vehicle_journey_idx) == Some(line_id) {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                    action_type,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a line {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_route<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        route_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_route_id(route_id) {
            return Err(DisruptionError::RouteAbsent(RouteId {
                id: route_id.to_string(),
            }));
        }
        for base_vehicle_journey_idx in base_model.vehicle_journeys() {
            if base_model.route_name(base_vehicle_journey_idx) == route_id {
                let vehicle_journey_id = base_model.vehicle_journey_name(base_vehicle_journey_idx);
                let result = self.apply_on_base_vehicle_journey(
                    base_model,
                    data,
                    vehicle_journey_id,
                    application_periods,
                    disruption_idx,
                    impact_idx,
                    action_type,
                );
                // we should never get a VehicleJourneyAbsent error
                if let Err(err) = result {
                    error!("Unexpected error while deleting a route {:?}", err);
                }
            }
        }
        Ok(())
    }

    fn apply_on_stop_area<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_area_id: &str,
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        if !base_model.contains_stop_area_id(stop_area_id) {
            return Err(DisruptionError::StopAreaAbsent(StopAreaId {
                id: stop_area_id.to_string(),
            }));
        }
        let mut concerned_stop_point = Vec::new();
        for stop_point in base_model.stop_points() {
            let stop_area_of_stop_point = base_model.stop_area_name(stop_point);
            if stop_area_id == stop_area_of_stop_point {
                let stop_point_id = base_model.stop_point_id(stop_point);
                concerned_stop_point.push(stop_point_id);
            }
        }
        let result = self.apply_on_stop_point(
            base_model,
            data,
            &concerned_stop_point,
            application_periods,
            disruption_idx,
            impact_idx,
            action_type,
        );
        if let Err(err) = result {
            error!("Error while deleting stop area {}. {:?}", stop_area_id, err);
        }
        Ok(())
    }

    fn apply_on_stop_point<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        stop_point_id: &[&str],
        application_periods: &TimePeriods,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) -> Result<(), DisruptionError> {
        let stop_point_idx: Vec<StopPointIdx> = stop_point_id
            .iter()
            .filter_map(|id| {
                let stop_point_idx = self.stop_point_idx(id, base_model);
                if stop_point_idx.is_none() {
                    let err = DisruptionError::StopPointAbsent(StopPointId { id: id.to_string() });
                    error!("Error while deleting stop point {}. {:?}", id, err);
                }
                stop_point_idx
            })
            .collect();

        for vehicle_journey_idx in base_model.vehicle_journeys() {
            let vehicle_journey_id = base_model.vehicle_journey_name(vehicle_journey_idx);
            if let Ok(base_stop_times) = base_model.stop_times(vehicle_journey_idx) {
                let contains_stop_point = base_stop_times
                    .clone()
                    .any(|stop_time| stop_point_idx.iter().any(|sp| sp == &stop_time.stop));
                if !contains_stop_point {
                    continue;
                }
                let timezone = base_model
                    .timezone(vehicle_journey_idx)
                    .unwrap_or(chrono_tz::UTC);
                for date in application_periods.dates_possibly_concerned() {
                    if let Some(time_period) =
                        base_model.trip_time_period(vehicle_journey_idx, &date)
                    {
                        if application_periods.intersects(&time_period) {
                            let is_stop_time_concerned = |stop_time: &super::StopTime| {
                                let concerned_stop_point =
                                    stop_point_idx.iter().any(|sp| sp == &stop_time.stop);
                                if !concerned_stop_point {
                                    return false;
                                }
                                let board_time =
                                    calendar::compose(&date, &stop_time.board_time, &timezone);
                                let debark_time =
                                    calendar::compose(&date, &stop_time.debark_time, &timezone);
                                application_periods.contains(&board_time)
                                    || application_periods.contains(&debark_time)
                            };

                            let stop_times: Vec<_> = base_stop_times
                                .clone()
                                .filter(|stop_time| !is_stop_time_concerned(stop_time))
                                .collect();

                            // if size changed it means that our vehicle is affected
                            // and need to be modified
                            if stop_times.len() != base_stop_times.len() {
                                self.dispatch_for_stop_point(
                                    base_model,
                                    data,
                                    vehicle_journey_id,
                                    &date,
                                    stop_times,
                                    disruption_idx,
                                    impact_idx,
                                    action_type,
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn dispatch_for_stop_point<Data: DataTrait + DataUpdate>(
        &mut self,
        base_model: &BaseModel,
        data: &mut Data,
        vehicle_journey_id: &str,
        date: &NaiveDate,
        stop_times: Vec<super::StopTime>,
        disruption_idx: &DisruptionIdx,
        impact_idx: &ImpactIdx,
        action_type: &ActionType,
    ) {
        match action_type {
            ActionType::ApplyImpact => {
                let result = self.modify_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    stop_times,
                    disruption_idx,
                    impact_idx,
                );
                if let Err(err) = result {
                    error!("Error while deleting stop point. {:?}", err);
                }
            }
            ActionType::ApplyInform => self.insert_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
            ActionType::CancelImpact => {
                let result = self.restore_base_trip(
                    base_model,
                    data,
                    vehicle_journey_id,
                    date,
                    disruption_idx,
                    impact_idx,
                );
                if let Err(err) = result {
                    error!(
                        "Unexpected error while restoring a base vehicle journey {:?}",
                        err
                    );
                }
            }
            ActionType::CancelInform => self.cancel_informed_linked_disruption(
                vehicle_journey_id,
                date,
                base_model,
                *disruption_idx,
                *impact_idx,
            ),
        }
    }
}
