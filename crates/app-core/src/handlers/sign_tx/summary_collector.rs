/*****************************************************************************
 *   Mintlayer Ledger App.
 *   (c) 2025 RBB S.r.l.
 *
 *  Licensed under the Apache License, Version 2.0 (the "License");
 *  you may not use this file except in compliance with the License.
 *  You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS,
 *  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 *****************************************************************************/

use alloc::collections::BTreeMap;

use crate::StatusWord;
use mintlayer_messages::{
    mlcp::{
        AccountCommand, AccountSpending, Amount, OrderAccountCommand, OutputValue, TxOutput, H256,
    },
    AdditionalOrderInfo, AdditionalUtxoInfo, TxInputWithAdditionalInfo,
};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub enum CoinOrTokenId {
    Coin,
    TokenId(H256),
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TxType {
    Transfer,
    Burn,
    Htlc,
    CreateDelegation,
    DelegationStake,
    DelegationWithdrawl,
    CreateStakePool,
    DecommissionStakePool,
    CreateNft,
    CreateToken,
    MintTokens,
    UnmintTokens,
    FreezeToken,
    UnfreezeToken,
    LockTokenSupply,
    ChangeTokenAuthority,
    ChangeTokenMetadataUri,
    FillOrder,
    FreezeOrder,
    CreateOrder,
    ConcludeOrder,
    DataDeposit,
    ComplexTransaction,
}

pub enum InputCommand {
    AccountSpending(AccountSpending),
    AccountCommand(AccountCommand),
    OrderCommand(OrderAccountCommand),
}

pub struct TxSummaryCollector {
    tx_type: Option<TxType>,
    input_command: Option<InputCommand>,
    total_inputs: BTreeMap<CoinOrTokenId, Amount>,
    total_outputs: BTreeMap<CoinOrTokenId, Amount>,
}

impl TxSummaryCollector {
    pub fn new() -> Self {
        Self {
            tx_type: None,
            input_command: None,
            total_inputs: BTreeMap::new(),
            total_outputs: BTreeMap::new(),
        }
    }

    pub fn input_command(&self) -> Option<&InputCommand> {
        self.input_command.as_ref()
    }

    pub fn tx_type(&self) -> Option<TxType> {
        self.tx_type
    }

    pub fn total_inputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_inputs
    }

    pub fn total_outputs(&self) -> &BTreeMap<CoinOrTokenId, Amount> {
        &self.total_outputs
    }

    pub fn fees_iter(
        &self,
    ) -> impl Iterator<Item = Result<(&CoinOrTokenId, u128), StatusWord>> + '_ {
        self.total_inputs()
            .iter()
            .map(move |(coin_or_token, amount)| {
                let out = *self
                    .total_outputs()
                    .get(coin_or_token)
                    .unwrap_or(&Amount::ZERO);

                let fee = amount
                    .into_atoms()
                    .checked_sub(out.into_atoms())
                    .ok_or(StatusWord::TxFeeUnderflow)?;

                Ok((coin_or_token, fee))
            })
    }

    pub fn process_output(&mut self, out: &TxOutput) -> Result<(), StatusWord> {
        match &out {
            TxOutput::Transfer(value, _) | TxOutput::LockThenTransfer(value, _, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Transfer);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::Burn(value) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Burn);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::Htlc(value, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::Htlc);

                let (coin_or_token_id, amount) = into_coin_or_token_id_and_amount(value)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
            TxOutput::CreateStakePool(_, data) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateStakePool);

                self.increase_output_totals(CoinOrTokenId::Coin, data.pledge)?;
            }
            TxOutput::ProduceBlockFromStake(_, _) => {}
            TxOutput::DelegateStaking(amount, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::DelegationStake);
                self.increase_output_totals(CoinOrTokenId::Coin, *amount)?;
            }
            TxOutput::CreateDelegationId(_, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateDelegation);
            }
            TxOutput::IssueFungibleToken(_) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateToken);
            }
            TxOutput::DataDeposit(_) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::DataDeposit);
            }
            TxOutput::IssueNft(_, _, _) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateNft);
            }
            TxOutput::CreateOrder(order_data) => {
                self.tx_type = merge_tx_type(self.tx_type, TxType::CreateOrder);
                let (coin_or_token_id, amount) =
                    into_coin_or_token_id_and_amount(&order_data.give)?;
                self.increase_output_totals(coin_or_token_id, amount)?;
            }
        }

        Ok(())
    }

    pub fn process_input(&mut self, inp: &TxInputWithAdditionalInfo) -> Result<(), StatusWord> {
        match inp {
            TxInputWithAdditionalInfo::Utxo(_, info) => match info {
                AdditionalUtxoInfo::UtxoWithPoolData {
                    utxo: _,
                    staker_balance,
                } => {
                    self.tx_type = merge_tx_type(self.tx_type, TxType::DecommissionStakePool);
                    self.increase_input_totals(CoinOrTokenId::Coin, *staker_balance)?;
                }
                AdditionalUtxoInfo::Utxo(utxo) => {
                    match &utxo {
                        TxOutput::Transfer(value, _)
                        | TxOutput::LockThenTransfer(value, _, _)
                        | TxOutput::Htlc(value, _) => {
                            let (coin_or_token_id, amount) =
                                into_coin_or_token_id_and_amount(value)?;
                            self.increase_input_totals(coin_or_token_id, amount)?;
                        }
                        TxOutput::Burn(_)
                        | TxOutput::ProduceBlockFromStake(_, _)
                        | TxOutput::CreateDelegationId(_, _)
                        | TxOutput::IssueFungibleToken(_)
                        | TxOutput::DataDeposit(_)
                        | TxOutput::DelegateStaking(_, _)
                        | TxOutput::CreateOrder(_) => return Err(StatusWord::TxInvalidInputUtxo),
                        TxOutput::CreateStakePool(_, data) => {
                            self.increase_input_totals(CoinOrTokenId::Coin, data.pledge)?;
                        }
                        TxOutput::IssueNft(nft_id, _, _) => {
                            self.increase_input_totals(
                                CoinOrTokenId::TokenId(*nft_id.hash()),
                                Amount::from_atoms(1),
                            )?;
                        }
                    };
                }
            },
            TxInputWithAdditionalInfo::Account(acc) => {
                self.input_command = Some(InputCommand::AccountSpending(acc.spending.clone()));
                match acc.spending {
                    AccountSpending::DelegationBalance(_, amount) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::DelegationWithdrawl);
                        self.increase_input_totals(CoinOrTokenId::Coin, amount)?;
                    }
                }
            }
            TxInputWithAdditionalInfo::AccountCommand(_, cmd) => {
                self.input_command = Some(InputCommand::AccountCommand(cmd.clone()));
                match cmd {
                    AccountCommand::MintTokens(token_id, amount) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::MintTokens);
                        self.increase_input_totals(
                            CoinOrTokenId::TokenId(*token_id.hash()),
                            *amount,
                        )?;
                    }
                    AccountCommand::ConcludeOrder(_) | AccountCommand::FillOrder(_, _, _) => {
                        return Err(StatusWord::OrdersV0NotSupported)
                    }
                    AccountCommand::UnmintTokens(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::UnmintTokens);
                    }
                    AccountCommand::LockTokenSupply(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::LockTokenSupply);
                    }
                    AccountCommand::FreezeToken(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::FreezeToken);
                    }
                    AccountCommand::UnfreezeToken(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::UnfreezeToken);
                    }
                    AccountCommand::ChangeTokenAuthority(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::ChangeTokenAuthority);
                    }
                    AccountCommand::ChangeTokenMetadataUri(_, _) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::ChangeTokenMetadataUri);
                    }
                }
            }
            TxInputWithAdditionalInfo::OrderAccountCommand(
                cmd,
                AdditionalOrderInfo {
                    initially_asked,
                    initially_given,
                    ask_balance,
                    give_balance,
                },
            ) => {
                self.input_command = Some(InputCommand::OrderCommand(cmd.clone()));
                match cmd {
                    OrderAccountCommand::FillOrder(_, fill_amount) => {
                        let (fill_coin_or_token_id, asked_amount) =
                            into_coin_or_token_id_and_amount(initially_asked)?;
                        let (given_coin_or_token_id, given_amount) =
                            into_coin_or_token_id_and_amount(initially_given)?;

                        self.increase_output_totals(fill_coin_or_token_id, *fill_amount)?;

                        let atoms = given_amount
                            .into_atoms()
                            .checked_mul(fill_amount.into_atoms())
                            .ok_or(StatusWord::TxNumericOperationFail)?
                            .checked_div(asked_amount.into_atoms())
                            .ok_or(StatusWord::TxNumericOperationFail)?;
                        let amount = Amount::from_atoms(atoms);
                        self.increase_input_totals(given_coin_or_token_id, amount)?;

                        self.tx_type = merge_tx_type(self.tx_type, TxType::FillOrder);
                    }
                    OrderAccountCommand::ConcludeOrder(_) => {
                        let (coin_or_token_id, _) =
                            into_coin_or_token_id_and_amount(initially_asked)?;
                        self.increase_input_totals(coin_or_token_id, *ask_balance)?;

                        let (coin_or_token_id, _) =
                            into_coin_or_token_id_and_amount(initially_given)?;
                        self.increase_input_totals(coin_or_token_id, *give_balance)?;

                        self.tx_type = merge_tx_type(self.tx_type, TxType::ConcludeOrder);
                    }
                    OrderAccountCommand::FreezeOrder(_) => {
                        self.tx_type = merge_tx_type(self.tx_type, TxType::FreezeOrder);
                    }
                }
            }
        };

        Ok(())
    }

    fn increase_input_totals(
        &mut self,
        key: CoinOrTokenId,
        amount: Amount,
    ) -> Result<(), StatusWord> {
        let total = self
            .total_inputs
            .entry(key)
            .or_insert(Amount::from_atoms(0));
        let new_total = total
            .into_atoms()
            .checked_add(amount.into_atoms())
            .ok_or(StatusWord::TxNumericOperationFail)?;
        *total = Amount::from_atoms(new_total);
        Ok(())
    }

    fn increase_output_totals(
        &mut self,
        key: CoinOrTokenId,
        amount: Amount,
    ) -> Result<(), StatusWord> {
        let total = self
            .total_outputs
            .entry(key)
            .or_insert(Amount::from_atoms(0));
        let new_total = total
            .into_atoms()
            .checked_add(amount.into_atoms())
            .ok_or(StatusWord::TxNumericOperationFail)?;
        *total = Amount::from_atoms(new_total);
        Ok(())
    }
}

fn merge_tx_type(tx_type: Option<TxType>, new_type: TxType) -> Option<TxType> {
    match tx_type {
        None => Some(new_type),
        // Transfers are a lower priority (as they can be change outputs) so keep the previous type
        Some(_) if new_type == TxType::Transfer => tx_type,
        Some(_) => Some(TxType::ComplexTransaction),
    }
}

fn into_coin_or_token_id_and_amount(
    value: &OutputValue,
) -> Result<(CoinOrTokenId, Amount), StatusWord> {
    match value {
        OutputValue::Coin(amount) => Ok((CoinOrTokenId::Coin, *amount)),
        OutputValue::TokenV1(token_id, amount) => {
            Ok((CoinOrTokenId::TokenId(*token_id.hash()), *amount))
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use crate::testing::prelude::*;

    use mintlayer_messages::mlcp;

    use super::*;

    // TODO: this is a sample test, need to expand it and add more tests
    #[test_item]
    fn sample_test() {
        let mut collector = TxSummaryCollector::new();

        collector.process_input(&TxInputWithAdditionalInfo::Utxo(
            mlcp::UtxoOutPoint::new(
                mlcp::OutPointSourceId::Transaction(mlcp::Id::new(mlcp::H256::zero())),
                0,
            ),
            AdditionalUtxoInfo::Utxo(mlcp::TxOutput::Transfer(
                mlcp::OutputValue::Coin(mlcp::Amount::from_atoms(123)),
                mlcp::Destination::AnyoneCanSpend,
            )),
        ));

        collector
            .process_output(&mlcp::TxOutput::Transfer(
                mlcp::OutputValue::Coin(mlcp::Amount::from_atoms(120)),
                mlcp::Destination::AnyoneCanSpend,
            ))
            .unwrap();

        let fees = collector
            .fees_iter()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(fees == [(&CoinOrTokenId::Coin, 3)]);
    }
}
