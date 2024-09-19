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

use alloy_consensus::{transaction::SignableTransaction, TxEnvelope, TypedTransaction};
use alloy_primitives::{address, Address, Bytes, Parity, Signature, U256};
use alloy_provider::Provider as AlloyProvider;
use alloy_rlp::Encodable;
use alloy_rpc_types_eth::TransactionRequest;
use alloy_transport::Transport;
use rundler_types::chain::{ChainSpec, L1GasOracleContractType};

use crate::ProviderResult;

pub(crate) mod v0_6;
pub(crate) mod v0_7;

mod arbitrum;
mod optimism;

#[derive(Debug, Default)]
enum L1GasOracle {
    ArbitrumNitro(Address),
    OptimismBedrock(Address),
    #[default]
    None,
}

impl L1GasOracle {
    fn new(chain_spec: &ChainSpec) -> L1GasOracle {
        match chain_spec.l1_gas_oracle_contract_type {
            L1GasOracleContractType::ArbitrumNitro => {
                L1GasOracle::ArbitrumNitro(chain_spec.l1_gas_oracle_contract_address)
            }
            L1GasOracleContractType::OptimismBedrock => {
                L1GasOracle::OptimismBedrock(chain_spec.l1_gas_oracle_contract_address)
            }
            L1GasOracleContractType::None => L1GasOracle::None,
        }
    }

    async fn estimate_l1_gas<AP: AlloyProvider<T>, T: Transport + Clone>(
        &self,
        provider: AP,
        to_address: Address,
        data: Bytes,
        gas_price: u128,
    ) -> ProviderResult<u128> {
        match self {
            L1GasOracle::ArbitrumNitro(oracle_address) => {
                arbitrum::estimate_l1_gas(provider, *oracle_address, to_address, data).await
            }
            L1GasOracle::OptimismBedrock(oracle_address) => {
                optimism::estimate_l1_gas(provider, *oracle_address, data, gas_price).await
            }
            L1GasOracle::None => Ok(0),
        }
    }
}

fn max_bundle_transaction_data(to_address: Address, data: Bytes, gas_price: u128) -> Bytes {
    // Fill in max values for unknown or varying fields
    let gas_price_ceil = gas_price.next_power_of_two() - 1; // max out bits of gas price, assume same power of 2
    let gas_limit = 0xffffffff; // 4 bytes
    let nonce = 0xffffffff; // 4 bytes
    let chain_id = 0xffffffff; // 4 bytes

    let tx = TransactionRequest::default()
        .from(address!("ffffffffffffffffffffffffffffffffffffffff"))
        .to(to_address)
        .gas_limit(gas_limit)
        .max_priority_fee_per_gas(gas_price_ceil)
        .max_fee_per_gas(gas_price_ceil)
        .value(U256::ZERO)
        .input(data.into())
        .nonce(nonce);

    // these conversions should not fail.
    let ty = tx.build_typed_tx().unwrap();
    let mut tx_1559 = match ty {
        TypedTransaction::Eip1559(tx) => tx,
        _ => {
            panic!("transaction is not eip1559");
        }
    };

    tx_1559.set_chain_id(chain_id);

    // use a max signature
    let tx_envelope: TxEnvelope = tx_1559
        .into_signed(Signature::new(
            U256::MAX,
            U256::MAX,
            Parity::Eip155(u64::MAX),
        ))
        .into();
    let mut encoded = vec![];
    tx_envelope.encode(&mut encoded);

    encoded.into()
}
