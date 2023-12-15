# Practical example

- **Dockerfile**: A Postgres database with prometheus_fdw installed
- **setup-prometheus-fdw.sql**: A script to show how to configure
- **setup-cache.sql**: Setup tables for local storage of metrics data
- **setup-metrics-sync.sql**: Automatically sync data from Prometheus to Postgres
- **run-demo.sh**: A script showing the whole process of running the demo

## Data model

This data model was inspired by the [Crunchy Data postgresql-prometheus-adapter](https://github.com/CrunchyData/postgresql-prometheus-adapter).

**metrics_labels**: Stores the metric name labels.

```
 id |               name                 |      labels
----+------------------------------------+-------------------------
  1 | container_cpu_usage_seconds_total  | {"pod": "my-pod-1", ...}
  2 | container_cpu_usage_seconds_total  | {"pod": "my-pod-2", ...}
  3 | container_memory_working_set_bytes | {"pod": "my-pod-1", ...}
  4 | container_memory_working_set_bytes | {"pod": "my-pod-2", ...}
```

**metrics_values**: A partitioned table that stores metric values, when they happened, and the corresponding labels.

```
 label_id |    time    |  value
----------+------------+----------
     4320 | 1702678142 | 12214272
     4320 | 1702678742 | 11923456
     4320 | 1702679342 | 12230656
     4320 | 1702679942 | 11804672
     4320 | 1702677542 | 11870208
     4331 | 1702679942 | 53743616
     4331 | 1702678142 | 54022144
     4331 | 1702678742 | 53903360
     4331 | 1702679342 | 53288960
     4331 | 1702677542 | 53514240
```
