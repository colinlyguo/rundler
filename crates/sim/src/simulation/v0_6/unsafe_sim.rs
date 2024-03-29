// This file is part of Rundler.
//
// Rundler is free software: you can redistribute it and/or modify it under the
// terms of the GNU Lesser General Public License as published by the Free Software
// Foundation, either version 3 of the License, or (at your option) any later version.
//
// Rundler is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
// See the GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with Rundler.
// If not, see https://www.gnu.org/licenses/.

use std::sync::Arc;

use ethers::types::H256;
use rundler_provider::{
    AggregatorOut, EntryPoint, Provider, SignatureAggregator, SimulationProvider,
};
use rundler_types::{
    pool::SimulationViolation, v0_6::UserOperation, EntityInfo, EntityInfos,
    UserOperation as UserOperationTrait, ValidTimeRange,
};

use crate::{
    SimulationError, SimulationResult, SimulationSettings as Settings, Simulator, ViolationError,
};

/// An unsafe simulator that can be used in place of a regular simulator
/// to extract the information needed from simulation while avoiding the use
/// of debug_traceCall.
///
/// WARNING: This is "unsafe" for a reason. None of the ERC-7562 checks are
/// performed.
pub struct UnsafeSimulator<P, E> {
    provider: Arc<P>,
    entry_point: E,
    sim_settings: Settings,
}

impl<P, E> UnsafeSimulator<P, E> {
    /// Creates a new unsafe simulator
    pub fn new(provider: Arc<P>, entry_point: E, sim_settings: Settings) -> Self {
        Self {
            provider,
            entry_point,
            sim_settings,
        }
    }
}

#[async_trait::async_trait]
impl<P, E> Simulator for UnsafeSimulator<P, E>
where
    P: Provider,
    E: EntryPoint
        + SimulationProvider<UO = UserOperation>
        + SignatureAggregator<UO = UserOperation>
        + Clone,
{
    type UO = UserOperation;

    // Run an unsafe simulation
    //
    // The only validation checks that are performed are signature checks
    async fn simulate_validation(
        &self,
        op: UserOperation,
        block_hash: Option<H256>,
        _expected_code_hash: Option<H256>,
    ) -> Result<SimulationResult, SimulationError> {
        tracing::info!("Performing unsafe simulation");

        let (block_hash, block_number) = match block_hash {
            // If we are given a block_hash, we return a None block number, avoiding an extra call
            Some(block_hash) => (block_hash, None),
            None => {
                let hash_and_num = self
                    .provider
                    .get_latest_block_hash_and_number()
                    .await
                    .map_err(anyhow::Error::from)?;
                (hash_and_num.0, Some(hash_and_num.1.as_u64()))
            }
        };

        // simulate the validation
        let validation_result = self
            .entry_point
            .call_simulate_validation(
                op.clone(),
                self.sim_settings.max_verification_gas,
                Some(block_hash),
            )
            .await
            .map_err(anyhow::Error::from)?;

        let pre_op_gas = validation_result.return_info.pre_op_gas;
        let valid_time_range = ValidTimeRange::new(
            validation_result.return_info.valid_after,
            validation_result.return_info.valid_until,
        );
        let requires_post_op = !validation_result.return_info.paymaster_context.is_empty();

        let entity_infos = EntityInfos {
            sender: EntityInfo {
                address: op.sender(),
                is_staked: false,
            },
            factory: op.factory().map(|f| EntityInfo {
                address: f,
                is_staked: false,
            }),
            paymaster: op.paymaster().map(|p| EntityInfo {
                address: p,
                is_staked: false,
            }),
            aggregator: validation_result.aggregator_info.map(|a| EntityInfo {
                address: a.address,
                is_staked: false,
            }),
        };

        let mut violations = vec![];

        let aggregator = if let Some(aggregator_info) = validation_result.aggregator_info {
            let agg_out = self
                .entry_point
                .validate_user_op_signature(
                    aggregator_info.address,
                    op,
                    self.sim_settings.max_verification_gas,
                )
                .await?;

            match agg_out {
                AggregatorOut::NotNeeded => None,
                AggregatorOut::SuccessWithInfo(info) => Some(info),
                AggregatorOut::ValidationReverted => {
                    violations.push(SimulationViolation::AggregatorValidationFailed);
                    None
                }
            }
        } else {
            None
        };

        if validation_result.return_info.account_sig_failed
            || validation_result.return_info.paymaster_sig_failed
        {
            violations.push(SimulationViolation::InvalidSignature);
        }

        if !violations.is_empty() {
            Err(SimulationError {
                violation_error: ViolationError::Violations(violations),
                entity_infos: Some(entity_infos),
            })?
        } else {
            Ok(SimulationResult {
                mempools: vec![H256::zero()],
                block_hash,
                block_number,
                pre_op_gas,
                valid_time_range,
                requires_post_op,
                entity_infos,
                aggregator,
                ..Default::default()
            })
        }
    }
}