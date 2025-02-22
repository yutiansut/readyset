//! Data types representing metrics dumped from a running ReadySet instance

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::Hash;

pub use metrics::Key;
use metrics_util::Histogram;
use serde::{Deserialize, Serialize};

/// A client for accessing readyset metrics for a deployment.
pub mod client;

/// Documents the set of metrics that are currently being recorded within
/// a ReadySet instance.
pub mod recorded {
    /// Counter: Set at startup of a ReadySet service to the current unix
    /// timestamp in milliseconds, this metric can be used to detect service
    /// restarts. When available, prefer the metrics scraped by whatever
    /// manages orchestration (such as kube-state-metrics's
    /// kube_pod_container_status_restarts metric)
    pub const NORIA_STARTUP_TIMESTAMP: &str = "startup_timestamp";

    /// Counter: The number of lookup misses that occured during replay
    /// requests. Recorded at the domain on every lookup miss during a
    /// replay request.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay miss is recorded in |
    /// | shard | The shard the replay miss is recorded in |
    /// | miss_in | The LocalNodeIndex of the data flow node where the miss occured |
    /// | needed_for | The client tag of the request that the replay is required for. |
    pub const DOMAIN_REPLAY_MISSES: &str = "domain.replay_misses";

    /// Histogram: The time in microseconds that a domain spends
    /// handling and forwarding a Message or Input packet. Recorded at
    /// the domain following handling each Message and Input packet.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | from_node | The src node of the packet. |
    /// | to_node |The dst node of the packet. |
    pub const DOMAIN_FORWARD_TIME: &str = "forward_time_us";

    /// Counter: The total time the domain spends handling and forwarding
    /// a Message or Input packet. Recorded at the domain following handling
    /// each Message and Input packet.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | from_node | The src node of the packet. |
    /// | to_node |The dst node of the packet. |
    pub const DOMAIN_TOTAL_FORWARD_TIME: &str = "total_forward_time_us";

    /// Histogram: The time in microseconds that a domain spends
    /// handling a ReplayPiece packet. Recorded at the domain following
    /// ReplayPiece packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay miss is recorded in. |
    /// | shard | The shard the replay miss is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_REPLAY_TIME: &str = "domain.handle_replay_time";

    /// Counter: The total time in microseconds that a domain spends
    /// handling a ReplayPiece packet. Recorded at the domain following
    /// ReplayPiece packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay miss is recorded in. |
    /// | shard | The shard the replay miss is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_TOTAL_REPLAY_TIME: &str = "domain.total_handle_replay_time";

    /// Histogram: The time in microseconds spent handling a reader replay
    /// request. Recorded at the domain following RequestReaderReplay
    /// packet handling.
    ///
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the reader replay request is recorded in. |
    /// | shard | The shard the reader replay request is recorded in. |
    /// | node | The LocalNodeIndex of the reader node handling the packet. |
    pub const DOMAIN_READER_REPLAY_REQUEST_TIME: &str = "domain.reader_replay_request_time_us";

    /// Counter: The total time in microseconds spent handling a reader replay
    /// request. Recorded at the domain following RequestReaderReplay
    /// packet handling.
    ///
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the reader replay request is recorded in. |
    /// | shard | The shard the reader replay request is recorded in. |
    /// | node | The LocalNodeIndex of the reader node handling the packet. |
    pub const DOMAIN_READER_TOTAL_REPLAY_REQUEST_TIME: &str =
        "domain.reader_total_replay_request_time_us";

    /// Histogram: The time in microseconds that a domain spends
    /// handling a RequestPartialReplay packet. Recorded at the domain
    /// following RequestPartialReplay packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard |The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_SEED_REPLAY_TIME: &str = "domain.seed_replay_time_us";

    /// Counter: The total time in microseconds that a domain spends
    /// handling a RequestPartialReplay packet. Recorded at the domain
    /// following RequestPartialReplay packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard |The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_TOTAL_SEED_REPLAY_TIME: &str = "domain.total_seed_replay_time_us";

    /// Histogram: The time in microseconds that a domain spawning a state
    /// chunker at a node during the processing of a StartReplay packet.
    /// Recorded at the domain when the state chunker thread is finished
    /// executing.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the start replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | from_node | The first node on the replay path. |
    pub const DOMAIN_CHUNKED_REPLAY_TIME: &str = "domain.chunked_replay_time_us";

    /// Counter: The total time in microseconds that a domain spawning a state
    /// chunker at a node during the processing of a StartReplay packet.
    /// Recorded at the domain when the state chunker thread is finished
    /// executing.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the start replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | from_node | The first node on the replay path. |
    pub const DOMAIN_TOTAL_CHUNKED_REPLAY_TIME: &str = "domain.total_chunked_replay_time_us";

    /// Histogram: The time in microseconds that a domain spends
    /// handling a StartReplay packet. Recorded at the domain
    /// following StartReplay packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_CHUNKED_REPLAY_START_TIME: &str = "domain.chunked_replay_start_time_us";

    /// Counter: The total time in microseconds that a domain spends
    /// handling a StartReplay packet. Recorded at the domain
    /// following StartReplay packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the replay is required for. |
    pub const DOMAIN_TOTAL_CHUNKED_REPLAY_START_TIME: &str =
        "domain.total_chunked_replay_start_time_us";

    /// Histogram: The time in microseconds that a domain spends
    /// handling a Finish packet for a replay. Recorded at the domain
    /// following Finish packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the Finish packet is required for. |
    pub const DOMAIN_FINISH_REPLAY_TIME: &str = "domain.finish_replay_time_us";

    /// Counter: The total time in microseconds that a domain spends
    /// handling a Finish packet for a replay. Recorded at the domain
    /// following Finish packet handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | tag | The client tag of the request that the Finish packet is required for. |
    pub const DOMAIN_TOTAL_FINISH_REPLAY_TIME: &str = "domain.total_finish_replay_time_us";

    /// Histogram: The time in microseconds that the domain spends handling
    /// a buffered replay request. Recorded at the domain following packet
    /// handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | requesting_shard | The shard that is requesting to be seeded. |
    /// | tag | The client tag of the request that the Finish packet is required for. |
    pub const DOMAIN_SEED_ALL_TIME: &str = "domain.seed_all_time_us";

    /// Counter: The total time in microseconds that the domain spends handling
    /// a buffered replay request. Recorded at the domain following packet
    /// handling.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain the replay request is recorded in. |
    /// | shard | The shard the replay request is recorded in. |
    /// | requesting_shard | The shard that is requesting to be seeded. |
    /// | tag | The client tag of the request that the Finish packet is required for. |
    pub const DOMAIN_TOTAL_SEED_ALL_TIME: &str = "domain.total_seed_all_time_us";

    /// Histogram: The amount of time spent handling an eviction
    /// request.
    pub const EVICTION_TIME: &str = "eviction_time_us";

    /// Histogram: The time in microseconds that the controller spent committing
    /// a migration to the soup graph. Recorded at the controller at the end of
    /// the `commit` call.
    pub const CONTROLLER_MIGRATION_TIME: &str = "controller.migration_time_us";

    /// Gauge: Migration in progress indicator. Set to 1 when a migration
    /// is in progress, 0 otherwise.
    pub const CONTROLLER_MIGRATION_IN_PROGRESS: &str = "controller.migration_in_progress";

    /// Counter: The number of evicitons performed at a worker. Incremented each
    /// time `do_eviction` is called at the worker.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The domain that the eviction is performed in. |
    pub const EVICTION_WORKER_EVICTIONS_REQUESTED: &str = "eviction_worker.evictions_requested";

    /// Gauge: The amount of memory allocated in the heap of the full server process
    pub const EVICTION_WORKER_HEAP_ALLOCATED_BYTES: &str = "eviction_worker.heap_allocated_bytes";

    /// Histogram: The amount of time that the eviction worker spends making an eviction
    /// decision and sending packets.
    pub const EVICTION_WORKER_EVICTION_TIME: &str = "eviction_worker.eviction_time_us";

    /// Gauge: The amount of bytes required to store a dataflow node's state./
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | node | The LocalNodeIndex of the dataflow node. |
    pub const NODE_STATE_SIZE_BYTES: &str = "node_state_size_bytes";

    /// Gauge: The sum of the amount of bytes used to store the dataflow node's
    /// partial state within a domain.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain. |
    /// | shard | The shard identifier of the domain. |
    pub const DOMAIN_PARTIAL_STATE_SIZE_BYTES: &str = "domain.partial_state_size_bytes";

    /// Gauge: The sum of the amount of bytes used to store a node's reader state
    /// within a domain.
    pub const READER_STATE_SIZE_BYTES: &str = "reader_state_size_bytes";

    /// Gauge: The sum of the amount of bytes used to store a node's base tables
    /// on disk.
    pub const ESTIMATED_BASE_TABLE_SIZE_BYTES: &str = "base_tables_estimated_size_bytes";

    /// Gauge: The sum of a domain's total node state and reader state bytes.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | domain | The index of the domain. |
    /// | shard | The shard identifier of the domain. |
    pub const DOMAIN_TOTAL_NODE_STATE_SIZE_BYTES: &str = "domain.total_node_state_size_bytes";

    /// Counter: The number of HTTP requests received at the readyset-server, for either the
    /// controller or worker.
    pub const SERVER_EXTERNAL_REQUESTS: &str = "server.external_requests";

    /// Counter: The number of worker HTTP requests received by the readyset-server.
    pub const SERVER_WORKER_REQUESTS: &str = "server.worker_requests";

    /// Counter: The number of controller HTTP requests received by the readyset-server.
    pub const SERVER_CONTROLLER_REQUESTS: &str = "server.controller_requests";

    /// Counter: The number of lookup requests to a base table nodes state.
    ///
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | table_name | The name of the base table. |
    /// | shard | The shard of the base table the lookup is requested in. |
    /// | node | The LocalNodeIndex of the base table node handling the packet. |
    pub const BASE_TABLE_LOOKUP_REQUESTS: &str = "base_table.lookup_requests";

    /// Counter: The number of packets dropped by an egress node.
    ///
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | node | The NodeIndex of the ingress node that was supposed to receive the packet. |
    pub const EGRESS_NODE_DROPPED_PACKETS: &str = "egress.dropped_packets";

    /// Counter: The number of packets sent by an egress node.
    ///
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | node | The NodeIndex of the ingress node were the packet was sent. |
    pub const EGRESS_NODE_SENT_PACKETS: &str = "egress.sent_packets";

    /// Counter: The number of eviction packets received.
    pub const EVICTION_REQUESTS: &str = "eviction_requests";

    /// Histogram: The total number of bytes evicted.
    pub const EVICTION_FREED_MEMORY: &str = "eviction_freed_memory";

    /// Counter: The number of times a query was served entirely from reader cache.
    pub const SERVER_VIEW_QUERY_HIT: &str = "server.view_query_result_hit";

    /// Counter: The number of times a query required at least a partial replay.
    pub const SERVER_VIEW_QUERY_MISS: &str = "server.view_query_result_miss";

    /// Histogram: The amount of time in microseconds spent waiting for an upquery during a read
    /// request.
    pub const SERVER_VIEW_UPQUERY_DURATION: &str = "server.view_query_upquery_duration_us";

    /// Counter: The number of times a dataflow node type is added to the
    /// dataflow graph. Recorded at the time the new graph is committed.
    ///
    /// | Tag | Description |
    /// | --- | ----------- |
    /// | ntype | The dataflow node type. |
    /// | node  | The index of the dataflow node. |
    pub const NODE_ADDED: &str = "node_added";

    /// Counter: The number of times a dataflow packet has been propagated
    /// for each domain.
    ///
    /// | Tag | Description |
    /// | domain | The index of the domain. |
    /// | shard | The shard of the base table the lookup is requested in. |
    /// | packet_type | The type of packet |
    pub const DOMAIN_PACKET_SENT: &str = "domain.packet_sent";

    /// Histogram: The time a snapshot takes to be performed.
    pub const REPLICATOR_SNAPSHOT_DURATION: &str = "replicator.snapshot_duration_us";

    /// How the replicator handled a snapshot.
    pub enum SnapshotStatusTag {
        /// A snapshot was started by the replicator.
        Started,
        /// A snapshot succeeded at the replicator.
        Successful,
        /// A snapshot failed at the replicator.
        Failed,
    }
    impl SnapshotStatusTag {
        /// Returns the enum tag as a &str for use in metrics labels.
        pub fn value(&self) -> &str {
            match self {
                SnapshotStatusTag::Started => "started",
                SnapshotStatusTag::Successful => "successful",
                SnapshotStatusTag::Failed => "failed",
            }
        }
    }

    /// Counter: Number of snapshots started at this node. Incremented by 1 when a
    /// snapshot begins.
    ///
    /// | Tag | Description |
    /// | status | SnapshotStatusTag |
    pub const REPLICATOR_SNAPSHOT_STATUS: &str = "replicator.snapshot_status";

    /// Gauge: Progress (in percent, 0-100) in snapshotting the given table
    ///
    /// | Tag | Description |
    /// | schema | Schema the relevant table exists in |
    /// | name | Name of the table being snapshot |
    pub const REPLICATOR_SNAPSHOT_PERCENT: &str = "replicator.snapshot.percent";

    /// Counter: Number of failures encountered when following the replication
    /// log.
    pub const REPLICATOR_FAILURE: &str = "replicator.update_failure";

    /// Counter: Number of tables that failed to replicate and are ignored
    pub const TABLE_FAILED_TO_REPLICATE: &str = "replicator.table_failed";

    /// Counter: Number of replication actions performed successfully.
    pub const REPLICATOR_SUCCESS: &str = "replicator.update_success";

    /// Gauge: Indicates whether a server is the leader. Set to 1 when the
    /// server is leader, 0 for follower.
    pub const CONTROLLER_IS_LEADER: &str = "controller.is_leader";

    /// Counter: The total amount of time spent servicing controller RPCs.
    ///
    /// | Tag | Description |
    /// | path | The http path associated with the rpc request. |
    pub const CONTROLLER_RPC_OVERALL_TIME: &str = "controller.rpc_overall_time";

    /// Histogram: The distribution of time spent servicing controller RPCs
    /// for each request.
    ///
    /// | Tag | Description |
    /// | path | The http path associated with the rpc request. |
    pub const CONTROLLER_RPC_REQUEST_TIME: &str = "controller.rpc_request_time";

    /// Histgoram: Write propagation time from binlog to reader node. For each
    /// input packet, this is recorded for each reader node that the packet
    /// propagates to. If the packet does not reach the reader because it hits a
    /// hole, the write propagation time is not recorded.
    pub const PACKET_WRITE_PROPAGATION_TIME: &str = "packet.write_propagation_time_us";

    /// Histogram: The time it takes to clone the dataflow state graph.
    pub const DATAFLOW_STATE_CLONE_TIME: &str = "dataflow_state.clone_time";

    /// Gauge: The size of the dataflow state, serialized and compressed, measured when it is
    /// written to the authority. This metric may be recorded even if the state does not
    /// get written to the authority (due to a failure). It is only recorded when the Consul
    /// authority is in use
    pub const DATAFLOW_STATE_SERIALIZED: &str = "dataflow_state.serialized_size";

    /// Gauge: A stub gague used to report the version information for the adapter.
    /// Labels are used to convey the version information.
    pub const READYSET_ADAPTER_VERSION: &str = "readyset.adapter_version";

    /// Gauge: A stub gague used to report the version information for the server.
    /// Labels are used to convey the version information.
    pub const READYSET_SERVER_VERSION: &str = "readyset.server_version";
}

/// A dumped metric's kind.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DumpedMetricValue {
    /// Counters that can be incremented or decremented
    Counter(f64),

    /// Gauges whose values can be explicitly set
    Gauge(f64),

    /// Histograms that track the number of samples that fall within
    /// predefined buckets.
    Histogram(Vec<(f64, u64)>),
}

/// A dumped metric's value.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DumpedMetric {
    /// Labels associated with this metric value.
    pub labels: HashMap<String, String>,
    /// The actual value.
    pub value: DumpedMetricValue,
}

/// A dump of metrics that implements `Serialize`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetricsDump {
    /// The actual metrics.
    pub metrics: HashMap<String, Vec<DumpedMetric>>,
}

fn convert_key(k: Key) -> (String, HashMap<String, String>) {
    let (name_parts, labels) = k.into_parts();
    let name = name_parts.as_str().to_string();
    let labels = labels
        .into_iter()
        .map(|l| {
            let (k, v) = l.into_parts();
            (k.into_owned(), v.into_owned())
        })
        .collect();
    (name, labels)
}

impl MetricsDump {
    /// Build a [`MetricsDump`] from a map containing values for counters, and another map
    /// containing values for gauges
    #[allow(clippy::mutable_key_type)] // for Key in the hashmap keys
    pub fn from_metrics(
        counters: Vec<(Key, u64)>,
        gauges: Vec<(Key, f64)>,
        histograms: Vec<(Key, Histogram)>,
    ) -> Self {
        let mut ret = HashMap::new();
        for (key, val) in counters.into_iter() {
            let (name, labels) = convert_key(key);
            let ent = ret.entry(name).or_insert_with(Vec::new);
            ent.push(DumpedMetric {
                labels,
                // It's going to be serialized to JSON anyway, so who cares
                value: DumpedMetricValue::Counter(val as f64),
            });
        }
        for (key, val) in gauges.into_iter() {
            let (name, labels) = convert_key(key);
            let ent = ret.entry(name).or_insert_with(Vec::new);
            ent.push(DumpedMetric {
                labels,
                value: DumpedMetricValue::Gauge(val),
            });
        }
        for (key, val) in histograms.into_iter() {
            let (name, labels) = convert_key(key);
            let ent = ret.entry(name).or_insert_with(Vec::new);
            ent.push(DumpedMetric {
                labels,
                value: DumpedMetricValue::Histogram(val.buckets()),
            });
        }

        Self { metrics: ret }
    }

    /// Return the sum of all the reported values for the given metric, if present
    pub fn total<K>(&self, metric: &K) -> Option<f64>
    where
        String: Borrow<K>,
        K: Hash + Eq + ?Sized,
    {
        Some(
            self.metrics
                .get(metric)?
                .iter()
                .map(|m| {
                    match &m.value {
                        DumpedMetricValue::Counter(v) | DumpedMetricValue::Gauge(v) => *v,
                        // Return the sum of counts for a histogram.
                        DumpedMetricValue::Histogram(v) => {
                            v.iter().map(|v| v.1).sum::<u64>() as f64
                        }
                    }
                })
                .sum(),
        )
    }

    /// Return an iterator over all the metric keys in this [`MetricsDump`]
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.metrics.keys()
    }

    /// Returns the first `DumpedMetricValue` found for a specific metric
    /// that includes `labels` as a subset of the metrics labels.
    ///
    /// None is returned if the metric is not found or there is no metric
    /// that includes `labels` as a subset.
    pub fn metric_with_labels<K>(
        &self,
        metric: &K,
        labels: &[(&K, &str)],
    ) -> Option<DumpedMetricValue>
    where
        String: Borrow<K>,
        K: Hash + Eq + ?Sized,
    {
        match self.metrics.get(metric) {
            None => {
                return None;
            }
            Some(dm) => {
                for m in dm {
                    if labels.iter().all(|l| match m.labels.get(l.0) {
                        None => false,
                        Some(metric_label) => l.1 == metric_label,
                    }) {
                        return Some(m.value.clone());
                    }
                }
            }
        }

        None
    }

    /// Returns the set of DumpedMetric's for a specific metric that have
    /// `labels` as a subset of their labels.
    ///
    /// An empty vec is returned if the metric is not found or there is no metric
    /// that includes `labels` as a subset.
    pub fn all_metrics_with_labels<K>(&self, metric: &K, labels: &[(&K, &str)]) -> Vec<DumpedMetric>
    where
        String: Borrow<K>,
        K: Hash + Eq + ?Sized,
    {
        let mut dumped_metrics = Vec::new();
        if let Some(dm) = self.metrics.get(metric) {
            for m in dm {
                if labels.iter().all(|l| {
                    m.labels
                        .get(l.0)
                        .filter(|metric_label| l.1 == *metric_label)
                        .is_some()
                }) {
                    dumped_metrics.push(m.clone());
                }
            }
        }

        dumped_metrics
    }
}

impl DumpedMetricValue {
    /// Get the encapsulated floating point value for the metric
    /// if it is not of the Histrogram type
    pub fn value(&self) -> Option<f64> {
        match self {
            DumpedMetricValue::Counter(v) => Some(*v),
            DumpedMetricValue::Gauge(v) => Some(*v),
            DumpedMetricValue::Histogram(_) => None,
        }
    }
}

/// Checks a metrics dump for a specified metric with a set of labels.
/// If the metric exists, returns Some(DumpedMetricValue), otherwise returns
/// None.
///
/// The first two parameters are MetricsDump, MetricsName. After which,
/// any number of labels can be specified. Labels are specified with the
/// syntax: label1 => label_value.
///
/// Example usage:
///     get_metric!(
///         metrics_dump,
///         recorded::DOMAIN_NODE_ADDED,
///         "ntype" => "Reader"
///     );
#[macro_export]
macro_rules! get_metric {
    (
        $metrics_dump:expr,
        $metrics_name:expr
        $(, $label:expr => $value:expr)*
    ) => {
        {
            let labels = vec![$(($label, $value),)*];
            $metrics_dump.metric_with_labels($metrics_name, labels.as_slice())
        }
    };
}

/// Checks a metrics dump for a specified metric with a set of labels.
/// If the metric exists, returns a vector of all metrics that match
/// the specified metrics that include the set of labels as a subset of
/// their own labels.
///
/// The first two parameters are MetricsDump, MetricsName. After which,
/// any number of labels can be specified. Labels are specified with the
/// syntax: label1 => label_value.
///
/// Example usage:
///     get_all_metrics!(
///         metrics_dump,
///         recorded::DOMAIN_NODE_ADDED,
///         "ntype" => "Reader"
///     );
#[macro_export]
macro_rules! get_all_metrics {
    (
        $metrics_dump:expr,
        $metrics_name:expr
        $(, $label:expr => $value:expr)*
    ) => {
        {
            #[allow(unused_mut)]
            let mut labels = Vec::new();
            $(
                labels.push(($label, $value));
            )*
            $metrics_dump.all_metrics_with_labels($metrics_name, labels.as_slice())
        }
    };
}

#[cfg(test)]
mod test {
    use super::*;

    // Tests the syntax of the get_metric macro.
    #[test]
    fn test_metric_macro() {
        let md = MetricsDump {
            metrics: HashMap::new(),
        };
        let metrics_name = recorded::SERVER_CONTROLLER_REQUESTS;
        assert_eq!(
            get_metric!(md, metrics_name, "test1" => "test2", "test3" => "test4"),
            None
        );
        assert_eq!(get_metric!(md, metrics_name, "test1" => "test2"), None);

        assert_eq!(get_metric!(md, metrics_name), None);
    }
}
