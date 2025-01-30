# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2025 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------

import asyncio

from nautilus_trader.core import nautilus_pyo3


async def run():
    http_client = nautilus_pyo3.TardisHttpClient()

    pyo3_instrument = await http_client.instrument("deribit", "okx")
    print(f"Received: {pyo3_instrument}")

    pyo3_instruments = await http_client.instruments("deribit")
    print(f"Received: {len(pyo3_instruments)} instruments")

    for inst in pyo3_instruments:
        print(inst.id)


if __name__ == "__main__":
    asyncio.run(run())
