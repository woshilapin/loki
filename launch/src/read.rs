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

use super::config;
use crate::{
    config::LaunchParams,
    loki::{timetables::TimetablesIter, TransitData},
};
use loki::{
    chrono::NaiveDate,
    timetables::Timetables as TimetablesTrait,
    tracing::{info, warn},
    transit_model::{self, Model},
    DataIO, DataTrait, LoadsData, PositiveDuration,
};
use std::{collections::BTreeMap, fmt::Debug, time::SystemTime};

pub fn read<Timetables>(
    launch_params: &config::LaunchParams,
) -> Result<(TransitData<Timetables>, Model), transit_model::Error>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    let model = read_model(launch_params)?;

    let loads_data = read_loads_data(launch_params, &model);

    let data = build_transit_data(
        &model,
        &loads_data,
        &launch_params.default_transfer_duration,
        None,
    );

    Ok((data, model))
}

pub fn read_model(
    launch_params: &LaunchParams,
) -> Result<transit_model::Model, transit_model::Error> {
    let model = match launch_params.input_data_type {
        config::InputDataType::Ntfs => transit_model::ntfs::read(&launch_params.input_data_path)?,
        config::InputDataType::Gtfs => {
            let configuration = transit_model::gtfs::Configuration {
                contributor: transit_model::objects::Contributor::default(),
                dataset: transit_model::objects::Dataset::default(),
                feed_infos: BTreeMap::new(),
                prefix_conf: None,
                on_demand_transport: false,
                on_demand_transport_comment: None,
                read_as_line: false,
            };
            info!("Reading gtfs from {:?}", &launch_params.input_data_path);
            use std::str::FromStr;
            let max_distance = f64::from_str(transit_model::TRANSFER_MAX_DISTANCE)?;
            let walking_speed = f64::from_str(transit_model::TRANSFER_WALKING_SPEED)?;
            let waiting_time = u32::from_str(transit_model::TRANSFER_WAITING_TIME)?;

            let model = transit_model::gtfs::Reader::new(configuration)
                .parse(&launch_params.input_data_path)?;
            transit_model::transfers::generates_transfers(
                model,
                max_distance,
                walking_speed,
                waiting_time,
                None,
            )?
        }
    };
    info!("Transit model loaded");
    Ok(model)
}

pub fn read_loads_data(launch_params: &LaunchParams, model: &transit_model::Model) -> LoadsData {
    launch_params
        .loads_data_path
        .as_ref()
        .map(|path| {
            LoadsData::new(&path, model).unwrap_or_else(|err| {
                warn!(
                    "Error while reading the passenger loads file at {:?} : {:?}",
                    &path,
                    err.source()
                );
                warn!("I'll use default loads.");
                LoadsData::empty()
            })
        })
        .unwrap_or_else(LoadsData::empty)
}

pub fn build_transit_data<Timetables>(
    model: &Model,
    loads_data: &LoadsData,
    default_transfer_duration: &PositiveDuration,
    restrict_calendar: Option<(NaiveDate, NaiveDate)>,
) -> TransitData<Timetables>
where
    Timetables: TimetablesTrait + for<'a> TimetablesIter<'a> + Debug,
{
    info!(
        "Number of vehicle journeys : {}",
        model.vehicle_journeys.len()
    );
    info!("Number of routes : {}", model.routes.len());

    let data_timer = SystemTime::now();
    let data = TransitData::new(
        model,
        loads_data,
        *default_transfer_duration,
        restrict_calendar,
    );
    let data_build_duration = data_timer.elapsed().unwrap().as_millis();
    info!("Data constructed in {} ms", data_build_duration);
    info!("Number of missions {} ", data.nb_of_missions());
    info!("Number of trips {} ", data.nb_of_trips());
    info!(
        "Validity dates between {} and {}",
        data.calendar().first_date(),
        data.calendar().last_date()
    );

    data
}
