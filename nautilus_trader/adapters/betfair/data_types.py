# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2021 Nautech Systems Pty Ltd. All rights reserved.
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

from enum import Enum
from typing import Dict

import orjson
import pyarrow as pa

from nautilus_trader.core.data import Data
from nautilus_trader.model.data.ticker import Ticker
from nautilus_trader.model.identifiers import InstrumentId
from nautilus_trader.model.objects import Price
from nautilus_trader.model.objects import Quantity
from nautilus_trader.model.orderbook.data import OrderBookDelta
from nautilus_trader.serialization.arrow.serializer import register_parquet
from nautilus_trader.serialization.base import register_serializable_object


class SubscriptionStatus(Enum):
    """
    Represents a `Betfair` subscription status.
    """

    UNSUBSCRIBED = 0
    PENDING_STARTUP = 1
    RUNNING = 2


class InstrumentSearch(Data):
    """
    Represents a `Betfair` instrument search.
    """

    def __init__(
        self,
        instruments,
        ts_event,
        ts_init,
    ):
        super().__init__(ts_event, ts_init)
        self.instruments = instruments


class BSPOrderBookDelta(OrderBookDelta):
    """
    Represents a `Betfair` BSP order book delta.
    """

    @staticmethod
    def from_dict(values):
        return BSPOrderBookDelta.from_dict(values)

    @staticmethod
    def to_dict(obj):
        return BSPOrderBookDelta.to_dict(obj)


class BetfairTicker(Ticker):
    """
    Represents a `Betfair` ticker.
    """

    def __init__(
        self,
        instrument_id: InstrumentId,
        ts_event: int,
        ts_init: int,
        last_traded_price: Price = None,
        traded_volume: Quantity = None,
        info=None,
    ):
        super().__init__(instrument_id=instrument_id, ts_event=ts_event, ts_init=ts_init, info=info)
        self.last_traded_price = last_traded_price
        self.traded_volume = traded_volume

    @classmethod
    def schema(cls):
        return pa.schema(
            {
                "instrument_id": pa.dictionary(pa.int8(), pa.string()),
                "ts_event": pa.int64(),
                "ts_init": pa.int64(),
                "last_traded_price": pa.string(),
                "traded_volume": pa.string(),
            },
            metadata={"type": "BetfairTicker"},
        )


def betfair_ticker_from_dict(values: Dict):
    return BetfairTicker(
        instrument_id=InstrumentId.from_str(values["instrument_id"]),
        ts_event=values["ts_event"],
        ts_init=values["ts_init"],
        last_traded_price=Price.from_str(values["last_traded_price"])
        if values["last_traded_price"]
        else None,
        traded_volume=Quantity.from_str(values["traded_volume"])
        if values["traded_volume"]
        else None,
        info=orjson.loads(values["info"]) if values.get("info") is not None else None,
    )


def betfair_ticker_to_dict(ticker: BetfairTicker):
    return {
        "type": type(ticker).__name__,
        "instrument_id": ticker.instrument_id.value,
        "ts_event": ticker.ts_event,
        "ts_init": ticker.ts_init,
        "last_traded_price": str(ticker.last_traded_price) if ticker.last_traded_price else None,
        "traded_volume": str(ticker.traded_volume) if ticker.traded_volume else None,
        "info": orjson.dumps(ticker.info) if ticker.info is not None else None,
    }


BSP_SCHEMA = pa.schema(
    {
        "instrument_id": pa.string(),
        "ts_event": pa.int64(),
        "ts_init": pa.int64(),
        "action": pa.string(),
        "order_side": pa.string(),
        "order_price": pa.float64(),
        "order_size": pa.float64(),
        "order_id": pa.string(),
        "book_type": pa.string(),
    },
    metadata={"type": "BSPOrderBookDelta"},
)


register_serializable_object(BetfairTicker, betfair_ticker_to_dict, betfair_ticker_from_dict)
register_parquet(cls=BetfairTicker, schema=BetfairTicker.schema())

register_serializable_object(
    BSPOrderBookDelta, BSPOrderBookDelta.to_dict, BSPOrderBookDelta.from_dict
)
register_parquet(cls=BSPOrderBookDelta, schema=BSP_SCHEMA)
