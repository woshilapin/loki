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

use failure::{format_err, Error};
use std::{
    fmt::Debug,
    ops::Deref,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;

use launch::{
    config,
    datetime::DateTimeRepresent,
    loki::{
        self,
        chrono::Utc,
        filters::Filters,
        model::{real_time::RealTimeModel, ModelRefs},
        request::generic_request,
        tracing::{debug, error, info, warn},
        transit_model::Model,
        NaiveDateTime, PositiveDuration, RequestInput, TransitData,
    },
    solver::Solver,
};
use std::convert::TryFrom;

use crate::{
    load_balancer::WorkerId,
    master_worker::{BaseTimetable, RealTimeTimetable},
    zmq_worker::{RequestMessage, ResponseMessage},
};

use super::{navitia_proto, response};

type BaseData = TransitData<BaseTimetable>;
type RealTimeData = TransitData<RealTimeTimetable>;

pub struct ComputeWorker {
    base_data_and_model: Arc<RwLock<(BaseData, Model)>>,
    real_time_data_and_model: Arc<RwLock<(RealTimeData, RealTimeModel)>>,
    solver: Solver,
    worker_id: WorkerId,
    request_default_params: config::RequestParams,
    request_channel: mpsc::Receiver<RequestMessage>,
    responses_channel: mpsc::Sender<(WorkerId, ResponseMessage)>,
}

impl ComputeWorker {
    pub fn new(
        worker_id: WorkerId,
        base_data_and_model: Arc<RwLock<(TransitData<BaseTimetable>, Model)>>,
        real_time_data_and_model: Arc<RwLock<(TransitData<RealTimeTimetable>, RealTimeModel)>>,
        request_default_params: config::RequestParams,
        responses_channel: mpsc::Sender<(WorkerId, ResponseMessage)>,
    ) -> (Self, mpsc::Sender<RequestMessage>) {
        let solver = Solver::new(0, 0);

        let (requests_channel_sender, requests_channel_receiver) = mpsc::channel(1);

        let result = Self {
            base_data_and_model,
            real_time_data_and_model,
            solver,
            worker_id,
            request_default_params,
            responses_channel,
            request_channel: requests_channel_receiver,
        };

        (result, requests_channel_sender)
    }

    pub fn run(mut self) -> Result<(), Error> {
        info!("Worker {} has launched.", self.worker_id.id);
        loop {
            // block on receiving message

            let has_request = self.request_channel.blocking_recv();

            info!("Worker {} received a request.", self.worker_id.id);

            let request_message = has_request.ok_or_else(|| {
                format_err!(
                    "Compute worker {} request channel is closed. This worker will stop.",
                    self.worker_id.id
                )
            })?;

            let proto_response = self
                .handle_request(request_message.payload)
                .map_err(|err| format_err!("This worker will stop. {}", err))?;

            let response_message = ResponseMessage {
                payload: proto_response,
                client_id: request_message.client_id,
            };

            debug!("Worker {} finished solving.", self.worker_id.id);

            // block until the response is sent
            self.responses_channel
                .blocking_send((self.worker_id, response_message))
                .map_err(|err| {
                    format_err!(
                        "Worker {} could not send response : {}. This worker will stop.",
                        self.worker_id.id,
                        err
                    )
                })?;

            debug!("Worker {} sent his response.", self.worker_id.id);
        }
    }

    fn handle_request(
        &mut self,
        proto_request: navitia_proto::Request,
    ) -> Result<navitia_proto::Response, Error> {
        let journeys_request_result = extract_journey_request(proto_request);
        match journeys_request_result {
            Err(err) => {
                // send a response saying that the journey request could not be handled
                warn!("Could not handle journey request : {}", err);
                Ok(make_error_response(err))
            }
            Ok(journeys_request) => {
                let real_time_level = journeys_request.realtime_level();
                match real_time_level {
                    navitia_proto::RtLevel::BaseSchedule
                    | navitia_proto::RtLevel::AdaptedSchedule => {
                        self.solve_on_base_schedule(journeys_request)
                    }
                    navitia_proto::RtLevel::Realtime => self.solve_on_real_time(journeys_request),
                }
            }
        }
    }

    fn solve_on_base_schedule(
        &mut self,
        journey_request: navitia_proto::JourneysRequest,
    ) -> Result<navitia_proto::Response, Error> {
        let rw_lock_read_guard = self
            .base_data_and_model
            .read()
            // if the read lock cannot be acquired, it means the lock is poisoned
            // and we return from this function and stop the thread
            .map_err(|err| {
                format_err!(
                    "Compute worker {} failed to acquire read lock on base_data_and_model. {}.",
                    self.worker_id.id,
                    err
                )
            })?;

        let (data, model) = rw_lock_read_guard.deref();
        let real_time_model = RealTimeModel::new();
        let model_refs = ModelRefs::new(model, &real_time_model);

        let solve_result = solve(
            journey_request,
            data,
            &model_refs,
            &mut self.solver,
            &self.request_default_params,
            config::ComparatorType::Basic,
        );

        let response = make_proto_response(solve_result, &model_refs);
        Ok(response)
        // RwLock is released
    }

    fn solve_on_real_time(
        &mut self,
        journey_request: navitia_proto::JourneysRequest,
    ) -> Result<navitia_proto::Response, Error> {
        let base_rw_lock_read_guard = self
            .base_data_and_model
            .read()
            // if the read lock cannot be acquired, it means the lock is poisoned
            // and we return from this function and stop the thread
            .map_err(|err| {
                format_err!(
                    "Compute worker {} failed to acquire read lock on base_data_and_model. {}.",
                    self.worker_id.id,
                    err
                )
            })?;

        let (_base_data, base_model) = base_rw_lock_read_guard.deref();

        let real_time_rw_lock_read_guard = self
            .real_time_data_and_model
            .read()
            // if the read lock cannot be acquired, it means the lock is poisoned
            // and we return from this function and stop the thread
            .map_err(|err| {
                format_err!(
                    "Compute worker {} failed to acquire read lock on real_time_data_and_model. {}.",
                    self.worker_id.id,
                    err
                )
            })?;

        let (real_time_data, real_time_model) = real_time_rw_lock_read_guard.deref();
        let model_refs = ModelRefs::new(base_model, real_time_model);

        let solve_result = solve(
            journey_request,
            real_time_data,
            &model_refs,
            &mut self.solver,
            &self.request_default_params,
            config::ComparatorType::Basic,
        );

        let response = make_proto_response(solve_result, &model_refs);
        Ok(response)
        // RwLocks are released
    }
}

use launch::loki::timetables::{Timetables as TimetablesTrait, TimetablesIter};

fn solve<Timetables>(
    journey_request: navitia_proto::JourneysRequest,
    data: &TransitData<Timetables>,
    model: &ModelRefs<'_>,
    solver: &mut Solver,
    request_default_params: &config::RequestParams,
    comparator_type: config::ComparatorType,
) -> Result<(RequestInput, Vec<loki::response::Response>), Error>
where
    Timetables: TimetablesTrait<
        Mission = generic_request::Mission,
        Position = generic_request::Position,
        Trip = generic_request::Trip,
    >,
    Timetables: for<'a> TimetablesIter<'a> + Debug,
    Timetables::Mission: 'static,
    Timetables::Position: 'static,
{
    // println!("{:#?}", journey_request);
    let departures_stop_point_and_fallback_duration = journey_request
        .origin
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th departure stop point {} has a fallback duration {} \
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context
                .place
                .strip_prefix("stop_point:")
                .map(|uri| uri.to_string())
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        })
        .collect();

    let arrivals_stop_point_and_fallback_duration = journey_request
        .destination
        .iter()
        .enumerate()
        .filter_map(|(idx, location_context)| {
            let duration = u32::try_from(location_context.access_duration)
                .map(|duration_u32| PositiveDuration::from_hms(0, 0, duration_u32))
                .ok()
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point {} has a fallback duration {}\
                        that cannot be converted to u32. I ignore it",
                        idx, location_context.place, location_context.access_duration
                    );
                    None
                })?;
            let stop_point_uri = location_context
                .place
                .strip_prefix("stop_point:")
                .map(|uri| uri.to_string())
                .or_else(|| {
                    warn!(
                        "The {}th arrival stop point has an uri {} \
                        that doesn't start with `stop_point:`. I ignore it",
                        idx, location_context.place,
                    );
                    None
                })?;
            // let trimmed = location_context.place.trim_start_matches("stop_point:");
            // let stop_point_uri = format!("StopPoint:{}", trimmed);
            // let stop_point_uri = location_context.place.clone();
            Some((stop_point_uri, duration))
        })
        .collect();

    let departure_timestamp_u64 = journey_request
        .datetimes
        .get(0)
        .ok_or_else(|| format_err!("Not departure datetime provided."))?;
    let departure_timestamp_i64 = i64::try_from(*departure_timestamp_u64).map_err(|_| {
        format_err!(
            "The departure datetime {} cannot be converted to a valid i64 timestamp.",
            departure_timestamp_u64
        )
    })?;
    let departure_datetime = loki::NaiveDateTime::from_timestamp(departure_timestamp_i64, 0);

    info!(
        "Requested timestamp {}, datetime {}",
        departure_timestamp_u64, departure_datetime
    );

    let max_journey_duration = u32::try_from(journey_request.max_duration)
        .map(|duration| PositiveDuration::from_hms(0, 0, duration))
        .unwrap_or_else(|_| {
            warn!(
                "The max duration {} cannot be converted to a u32.\
                I'm gonna use the default {} as max duration",
                journey_request.max_duration, request_default_params.max_journey_duration
            );
            request_default_params.max_journey_duration
        });

    let max_nb_of_legs = u8::try_from(journey_request.max_transfers + 1).unwrap_or_else(|_| {
        warn!(
            "The max nb of transfers {} cannot be converted to a u8.\
                    I'm gonna use the default {} as the max nb of legs",
            journey_request.max_transfers, request_default_params.max_nb_of_legs
        );
        request_default_params.max_nb_of_legs
    });

    let data_filters = Filters::new(
        model,
        &journey_request.forbidden_uris,
        &journey_request.allowed_id,
    );

    let request_input = RequestInput {
        datetime: departure_datetime,
        departures_stop_point_and_fallback_duration,
        arrivals_stop_point_and_fallback_duration,
        leg_arrival_penalty: request_default_params.leg_arrival_penalty,
        leg_walking_penalty: request_default_params.leg_walking_penalty,
        max_nb_of_legs,
        max_journey_duration,
        too_late_threshold: request_default_params.too_late_threshold,
    };

    let datetime_represent = match journey_request.clockwise {
        true => DateTimeRepresent::Departure,
        false => DateTimeRepresent::Arrival,
    };
    // trace!("{:#?}", request_input);

    let responses = solver.solve_request(
        data,
        model,
        &request_input,
        data_filters,
        &comparator_type,
        &datetime_represent,
    )?;
    for response in responses.iter() {
        debug!("{}", response.print(model)?);
    }
    Ok((request_input, responses))
}

fn make_proto_response(
    solve_result: Result<(RequestInput, Vec<loki::Response>), Error>,
    model: &ModelRefs<'_>,
) -> navitia_proto::Response {
    match solve_result {
        Result::Err(err) => {
            error!("Error while solving request : {}", err);
            make_error_response(err)
        }
        Ok((request_input, journeys)) => {
            let response_result = response::make_response(&request_input, journeys, model);
            match response_result {
                Result::Err(err) => {
                    error!(
                        "Error while encoding protobuf response for request : {}",
                        err
                    );
                    make_error_response(err)
                }
                Ok(resp) => {
                    // trace!("{:#?}", resp);
                    resp
                }
            }
        }
    }
}

fn make_error_response(error: Error) -> navitia_proto::Response {
    let mut proto_response = navitia_proto::Response::default();
    proto_response.set_response_type(navitia_proto::ResponseType::NoSolution);
    let mut proto_error = navitia_proto::Error::default();
    proto_error.set_id(navitia_proto::error::ErrorId::InternalError);
    proto_error.message = Some(format!("{}", error));
    proto_response.error = Some(proto_error);
    proto_response
}

fn extract_journey_request(
    proto_request: navitia_proto::Request,
) -> Result<navitia_proto::JourneysRequest, Error> {
    if let Some(deadline_str) = proto_request.deadline {
        let datetime_result = NaiveDateTime::parse_from_str(&deadline_str, "%Y%m%dT%H%M%S,%f");
        match datetime_result {
            Ok(datetime) => {
                let now = Utc::now().naive_utc();
                if now > datetime {
                    return Err(format_err!("Deadline reached."));
                }
            }
            Err(err) => {
                warn!(
                    "Could not parse deadline string {}. Error : {}",
                    deadline_str, err
                );
            }
        }
    }
    proto_request
        .journeys
        .ok_or_else(|| format_err!("request.journey should not be empty for api PtPlanner."))
}
