SELECT
    ml.labels ->> 'pod' AS pod_name,
    mv.value / 1048576.0 AS memory_usage_mib
FROM
    metric_labels ml
JOIN
    metric_values mv ON ml.id = mv.label_id
WHERE
    ml.name = 'container_memory_working_set_bytes' AND
    ml.labels ->> 'namespace' = 'kube-system' AND
    mv.time = (
        SELECT MAX(mv2.time)
        FROM metric_values mv2
        WHERE mv2.label_id = ml.id
    )
ORDER BY
    mv.time DESC;
