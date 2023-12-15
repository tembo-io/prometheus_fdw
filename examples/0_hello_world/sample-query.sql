SELECT
  *
FROM metrics
WHERE
  metric_name = 'container_cpu_usage_seconds_total'
  AND metric_time > :start_time
  AND metric_time < :end_time;
