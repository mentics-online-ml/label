#!/usr/bin/env bash

docker run --name scylla -d scylladb/scylla --developer-mode 1

# INFO  2024-04-27 01:29:27,352 [shard  0:stat] cql_server_controller - Starting listening for CQL clients on 172.17.0.2:9042 (unencrypted, non-shard-aware)
# INFO  2024-04-27 01:29:27,352 [shard  0:stat] cql_server_controller - Starting listening for CQL clients on 172.17.0.2:19042 (unencrypted, shard-aware)