# Build the image and run it
docker build -t example-local-image .
docker rm --force local-tembo
docker run -d -it --name local-tembo -p 5432:5432 --rm example-local-image

# wait for connect
until psql postgres://postgres:postgres@localhost:5432 -c "select 1" &> /dev/null; do
  echo "Waiting for postgres to start..."
  sleep 1
done
echo "Ready!"

# Run sample scripts
psql postgres://postgres:postgres@localhost:5432 -f ./setup-prometheus-fdw.sql

start_time=$(date -j -v-500S +%s)
end_time=$(date -j +%s)
psql postgres://postgres:postgres@localhost:5432 -v start_time=$start_time -v end_time=$end_time -f ./sample-query.sql
