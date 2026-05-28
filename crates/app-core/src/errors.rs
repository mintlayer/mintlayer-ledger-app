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

use ledger_device_sdk::ecc::CxError;
use messages::StatusWord;

pub fn cx_err_to_status(e: CxError) -> StatusWord {
    match e {
        CxError::Carry => StatusWord::EccCarry,
        CxError::Locked => StatusWord::EccLocked,
        CxError::Unlocked => StatusWord::EccUnlocked,
        CxError::NotLocked => StatusWord::EccNotLocked,
        CxError::NotUnlocked => StatusWord::EccNotUnlocked,
        CxError::InternalError => StatusWord::EccInternalError,
        CxError::InvalidParameterSize => StatusWord::EccInvalidParameterSize,
        CxError::InvalidParameterValue => StatusWord::EccInvalidParameterValue,
        CxError::InvalidParameter => StatusWord::EccInvalidParameter,
        CxError::NotInvertible => StatusWord::EccNotInvertible,
        CxError::Overflow => StatusWord::EccOverflow,
        CxError::MemoryFull => StatusWord::EccMemoryFull,
        CxError::NoResidue => StatusWord::EccNoResidue,
        CxError::PointAtInfinity => StatusWord::EccPointAtInfinity,
        CxError::InvalidPoint => StatusWord::EccInvalidPoint,
        CxError::InvalidCurve => StatusWord::EccInvalidCurve,
        CxError::GenericError => StatusWord::EccGenericError,
    }
}
