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

mod context;
pub(crate) use context::ValidationContextProvider;

mod tracer;

/// Required buffer for verification gas limit when targeting the 0.6 entrypoint contract
pub(crate) const REQUIRED_VERIFICATION_GAS_LIMIT_BUFFER: u128 = 2000;
