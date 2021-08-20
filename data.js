window.BENCHMARK_DATA = {
  "lastUpdate": 1629462547244,
  "repoUrl": "https://github.com/CanalTP/loki",
  "entries": {
    "Loki Benchmark": [
      {
        "commit": {
          "author": {
            "email": "hicham.azimani@kisio.com",
            "name": "HichamAz",
            "username": "AzHicham"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "96353e1ccdcb563cd76b256ae012fb15053b5e86",
          "message": "Merge pull request #58 from CanalTP/bench\n\n[CI] Benchmark",
          "timestamp": "2021-08-20T12:46:50+02:00",
          "tree_id": "633ac37201f376d65337c8d18310c803515ddf8e",
          "url": "https://github.com/CanalTP/loki/commit/96353e1ccdcb563cd76b256ae012fb15053b5e86"
        },
        "date": 1629456860841,
        "tool": "cargo",
        "benches": [
          {
            "name": "routing_loads_bench",
            "value": 21142,
            "range": "± 2106",
            "unit": "ns/iter"
          },
          {
            "name": "setup_routing_basic_bench",
            "value": 20755,
            "range": "± 4228",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "hicham.azimani@kisio.com",
            "name": "HichamAz",
            "username": "AzHicham"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "da664dfdf2122e682b3c27fb7c96584887964721",
          "message": "Merge pull request #57 from CanalTP/second-pass\n\nOptimize Journey - Second pass",
          "timestamp": "2021-08-20T14:23:09+02:00",
          "tree_id": "b4e64e3c7845a02163c9d51932e54d2ba5be9168",
          "url": "https://github.com/CanalTP/loki/commit/da664dfdf2122e682b3c27fb7c96584887964721"
        },
        "date": 1629462546633,
        "tool": "cargo",
        "benches": [
          {
            "name": "routing_loads_bench",
            "value": 21425,
            "range": "± 228",
            "unit": "ns/iter"
          },
          {
            "name": "setup_routing_basic_bench",
            "value": 21446,
            "range": "± 278",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}