
## Dockers
Provides dockers to run a fully featured navitia, where loki_server
will solve the "public transport" part of the request, instead of kraken.


# Build
You can build the dockers by launching
```bash
./build_docker.sh -o my_github_token
```
from the root directory of this repository, where `my_github_token` is a OAuth token for github.

# Binarize

Put gtfs or ntfs datasets in `./data/` with one folder per instance.
You can also add osm data.
In the following example, we have two datasets (corsese and idfm), with osm data provided only for corse.

```
.
├── corse
│   ├── gtfs
│   │   ├── agency.txt
│   │   ├── calendar_dates.txt
│   │   ├── calendar.txt
│   │   ├── log_GTFS.txt
│   │   ├── routes.txt
│   │   ├── shapes.txt
│   │   ├── stops.txt
│   │   ├── stop_times.txt
│   │   └── trips.txt
│   ├── osm
│   │   └── corse-latest.osm.pbf
│   └── stoptimes_loads.csv
├── idfm
│   ├── ntfs
│   │   ├── calendar.txt
│   │   ├── comment_links.txt
│   │   ├── comments.txt
│   │   ├── commercial_modes.txt
│   │   ├── companies.txt
│   │   ├── contributors.txt
│   │   ├── datasets.txt
│   │   ├── equipments.txt
│   │   ├── feed_infos.txt
│   │   ├── lines.txt
│   │   ├── networks.txt
│   │   ├── object_codes.txt
│   │   ├── physical_modes.txt
│   │   ├── routes.txt
│   │   ├── stops.txt
│   │   ├── stop_times.txt
│   │   ├── transfers.txt
│   │   ├── trip_properties.txt
│   │   └── trips.txt
│   └── stoptimes_loads.csv
```

Then, from the root directory of this repository, run :

```bash
docker run --rm -v "$PWD":/storage -v /var/run/docker.sock:/var/run/docker.sock   navitia/mc_bina
```

This will create a folder `./mc_navitia` containing everything needed to launch navitia.

# Launch docker

In `./mc_navitia` run
```bash
docker-compose up
```

Then you can send http requests to the jormun server on http://localhost:9191 !

Don't forget to add "_override_scenario=distributed" to your requests !
Otherwise you won't be using the loki server.

# Launch kubernetes


```bash
# start the cluster
minikube start
# mount data into the cluster only node
minikube mount ./mc_navitia/:/data
# create a persistent volume with `storageClassName: storage-class-navitia`
kubectl apply -f ./docker/kubernetes-volume.yml
# create all navitia services, they will mount the data present in the persitent volume we just created
kubectl apply -f ./mc_navitia/kubernetes.yml
# forward port from localhost into the cluster to the navitia service
kubectl port-forward service/navitia 9192:80
# navitia is responding on http://localhost:9192/ !
```
