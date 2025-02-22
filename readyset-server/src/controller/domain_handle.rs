use std::collections::HashMap;

use array2::Array2;
use dataflow::prelude::*;
use dataflow::DomainRequest;
use futures::{stream, StreamExt, TryStreamExt};
use serde::de::DeserializeOwned;
use tracing::error;

use crate::controller::{Worker, WorkerIdentifier};
use crate::worker::WorkerRequestKind;

/// A `DomainHandle` is a handle that allows communicating with all of the replicas and shards of a
/// given domain.
#[derive(Clone)]
pub(super) struct DomainHandle {
    idx: DomainIndex,
    /// Maps from shard index, to replica index, to address of the worker running that replica of
    /// that shard of the domain
    shards: Array2<WorkerIdentifier>,
}

impl DomainHandle {
    pub fn new(idx: DomainIndex, shards: Array2<WorkerIdentifier>) -> Self {
        Self { idx, shards }
    }

    pub(super) fn index(&self) -> DomainIndex {
        self.idx
    }

    pub(super) fn shards(&self) -> impl Iterator<Item = &[WorkerIdentifier]> {
        self.shards.rows()
    }

    /// Returns the number of times this domain is sharded
    pub(super) fn num_shards(&self) -> usize {
        self.shards.num_rows()
    }

    /// Returns the number of times this domain is replicated
    pub(super) fn num_replicas(&self) -> usize {
        self.shards.row_size()
    }

    /// Look up which worker the given shard/replica pair is assigned to
    ///
    /// Returns [`ReadySetError::NoSuchReplica`] if the replica has not been assigned to a worker.
    pub(super) fn assignment(
        &self,
        shard: usize,
        replica: usize,
    ) -> ReadySetResult<&WorkerIdentifier> {
        self.shards
            .get((shard, replica))
            .ok_or_else(|| ReadySetError::NoSuchReplica {
                domain_index: self.idx.index(),
                shard,
                replica,
            })
    }

    pub(super) fn is_assigned_to_worker(&self, worker: &WorkerIdentifier) -> bool {
        self.shards.cells().iter().any(|s| s == worker)
    }

    pub(super) async fn send_to_healthy_shard_replica<R>(
        &self,
        shard: usize,
        replica: usize,
        req: DomainRequest,
        workers: &HashMap<WorkerIdentifier, Worker>,
    ) -> ReadySetResult<R>
    where
        R: DeserializeOwned,
    {
        let addr = self.assignment(shard, replica)?;
        if let Some(worker) = workers.get(addr) {
            let req = req.clone();
            let replica_address = ReplicaAddress {
                domain_index: self.idx,
                shard,
                replica,
            };
            Ok(worker
                .rpc(WorkerRequestKind::DomainRequest {
                    replica_address,
                    request: Box::new(req),
                })
                .await
                .map_err(|e| {
                    rpc_err_no_downcast(format!("domain request to {}", replica_address), e)
                })?)
        } else {
            error!(%addr, ?req, "tried to send domain request to failed worker");
            Err(ReadySetError::WorkerFailed { uri: addr.clone() })
        }
    }

    pub(super) async fn send_to_healthy_shard<R>(
        &self,
        shard: usize,
        req: DomainRequest,
        workers: &HashMap<WorkerIdentifier, Worker>,
    ) -> ReadySetResult<Vec<R>>
    where
        R: DeserializeOwned,
    {
        stream::iter(0..self.num_replicas())
            .then(move |replica| {
                self.send_to_healthy_shard_replica(shard, replica, req.clone(), workers)
            })
            .try_collect()
            .await
    }

    /// returns shard first, then replica
    pub(super) async fn send_to_healthy<R>(
        &self,
        req: DomainRequest,
        workers: &HashMap<WorkerIdentifier, Worker>,
    ) -> ReadySetResult<Vec<Vec<R>>>
    where
        R: DeserializeOwned,
    {
        stream::iter(0..self.num_shards())
            .then(move |shard| {
                let req = req.clone();
                stream::iter(0..self.num_replicas())
                    .then(move |replica| {
                        self.send_to_healthy_shard_replica(shard, replica, req.clone(), workers)
                    })
                    .try_collect()
            })
            .try_collect()
            .await
    }
}
