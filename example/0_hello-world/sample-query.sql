SELECT
  *
FROM metrics
WHERE
  metric_name = 'container_cpu_usage_seconds_total'
  AND metric_time > EXTRACT(EPOCH FROM NOW() - INTERVAL '500 seconds')
  AND metric_time < EXTRACT(EPOCH FROM NOW());
