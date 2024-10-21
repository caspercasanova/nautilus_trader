// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2024 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

use chrono::{DateTime, Utc};
use nautilus_core::nanos::UnixNanos;
use nautilus_model::{
    data::{
        bar::{Bar, BarType},
        delta::OrderBookDelta,
        deltas::{OrderBookDeltas, OrderBookDeltas_API},
        order::BookOrder,
        trade::TradeTick,
        Data,
    },
    enums::{AggregationSource, OrderSide, RecordFlag},
    identifiers::{InstrumentId, TradeId},
    types::{price::Price, quantity::Quantity},
};
use uuid::Uuid;

use super::{
    enums::WsMessage,
    message::{BarMsg, BookChangeMsg, BookLevel, BookSnapshotMsg, TradeMsg},
};
use crate::tardis::parse::{
    parse_aggressor_side, parse_bar_spec, parse_book_action, parse_instrument_id,
};

#[must_use]
pub fn parse_tardis_ws_message(
    msg: WsMessage,
    price_precision: u8,
    size_precision: u8,
) -> Option<Data> {
    match msg {
        WsMessage::BookChange(msg) => Some(Data::Deltas(parse_book_change_msg(
            msg,
            price_precision,
            size_precision,
            None, // Instrument ID handling TBD
        ))),
        WsMessage::BookSnapshot(msg) => Some(Data::Deltas(parse_book_snapshot_msg(
            msg,
            price_precision,
            size_precision,
            None, // Instrument ID handling TBD
        ))),
        WsMessage::Trade(msg) => Some(Data::Trade(parse_trade_msg(
            msg,
            price_precision,
            size_precision,
            None, // Instrument ID handling TBD
        ))),
        WsMessage::Bar(msg) => Some(Data::Bar(parse_bar_msg(
            msg,
            price_precision,
            size_precision,
            None, // Instrument ID handling TBD
        ))),
        WsMessage::DerivativeTicker(_) => None,
        WsMessage::Disconnect(_) => None,
    }
}

#[must_use]
pub fn parse_book_change_msg(
    msg: BookChangeMsg,
    price_precision: u8,
    size_precision: u8,
    instrument_id: Option<InstrumentId>,
) -> OrderBookDeltas_API {
    let temp_exchange_str = serde_json::to_string(&msg.exchange)
        .unwrap()
        .trim_matches('"')
        .to_string();

    let instrument_id = match &instrument_id {
        Some(id) => *id,
        None => parse_instrument_id(&temp_exchange_str, &msg.symbol),
    };

    parse_book_msg(
        msg.bids,
        msg.asks,
        msg.is_snapshot,
        price_precision,
        size_precision,
        instrument_id,
        msg.timestamp,
        msg.local_timestamp,
    )
}

#[must_use]
pub fn parse_book_snapshot_msg(
    msg: BookSnapshotMsg,
    price_precision: u8,
    size_precision: u8,
    instrument_id: Option<InstrumentId>,
) -> OrderBookDeltas_API {
    let temp_exchange_str = serde_json::to_string(&msg.exchange)
        .unwrap()
        .trim_matches('"')
        .to_string();

    let instrument_id = match &instrument_id {
        Some(id) => *id,
        None => parse_instrument_id(&temp_exchange_str, &msg.symbol),
    };

    parse_book_msg(
        msg.bids,
        msg.asks,
        true,
        price_precision,
        size_precision,
        instrument_id,
        msg.timestamp,
        msg.local_timestamp,
    )
}

#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn parse_book_msg(
    bids: Vec<BookLevel>,
    asks: Vec<BookLevel>,
    is_snapshot: bool,
    price_precision: u8,
    size_precision: u8,
    instrument_id: InstrumentId,
    timestamp: DateTime<Utc>,
    local_timestamp: DateTime<Utc>,
) -> OrderBookDeltas_API {
    let ts_event = UnixNanos::from(timestamp.timestamp_nanos_opt().unwrap() as u64);
    let ts_init = UnixNanos::from(local_timestamp.timestamp_nanos_opt().unwrap() as u64);

    let mut deltas: Vec<OrderBookDelta> = Vec::with_capacity(bids.len() + asks.len());

    for level in bids {
        deltas.push(parse_book_level(
            instrument_id,
            price_precision,
            size_precision,
            OrderSide::Buy,
            level,
            is_snapshot,
            ts_event,
            ts_init,
        ));
    }

    for level in asks {
        deltas.push(parse_book_level(
            instrument_id,
            price_precision,
            size_precision,
            OrderSide::Sell,
            level,
            is_snapshot,
            ts_event,
            ts_init,
        ));
    }

    if let Some(last_delta) = deltas.last_mut() {
        last_delta.flags += RecordFlag::F_LAST.value();
    }

    // TODO: Opaque pointer wrapper necessary for Cython (remove once Cython gone)
    OrderBookDeltas_API::new(OrderBookDeltas::new(instrument_id, deltas))
}

#[allow(clippy::too_many_arguments)]
#[must_use]
pub fn parse_book_level(
    instrument_id: InstrumentId,
    price_precision: u8,
    size_precision: u8,
    side: OrderSide,
    level: BookLevel,
    is_snapshot: bool,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
) -> OrderBookDelta {
    let action = parse_book_action(is_snapshot, level.amount);
    let price = Price::new(level.price, price_precision);
    let size = Quantity::new(level.amount, size_precision);
    let order_id = 0; // Not applicable for L2 data
    let order = BookOrder::new(side, price, size, order_id);
    let flags = if is_snapshot {
        RecordFlag::F_SNAPSHOT.value()
    } else {
        0
    };
    let sequence = 0; // Not available

    OrderBookDelta::new(
        instrument_id,
        action,
        order,
        flags,
        sequence,
        ts_event,
        ts_init,
    )
}

#[must_use]
pub fn parse_trade_msg(
    msg: TradeMsg,
    price_precision: u8,
    size_precision: u8,
    instrument_id: Option<InstrumentId>,
) -> TradeTick {
    let temp_exchange_str = serde_json::to_string(&msg.exchange)
        .unwrap()
        .trim_matches('"')
        .to_string();

    let instrument_id = match &instrument_id {
        Some(id) => *id,
        None => parse_instrument_id(&temp_exchange_str, &msg.symbol),
    };

    let price = Price::new(msg.price, price_precision);
    let size = Quantity::new(msg.amount, size_precision);
    let aggressor_side = parse_aggressor_side(&msg.side);
    let trade_id = TradeId::new(&msg.id.unwrap_or_else(|| Uuid::new_v4().to_string()));
    let ts_event = UnixNanos::from(msg.timestamp.timestamp_nanos_opt().unwrap() as u64);
    let ts_init = UnixNanos::from(msg.local_timestamp.timestamp_nanos_opt().unwrap() as u64);

    TradeTick::new(
        instrument_id,
        price,
        size,
        aggressor_side,
        trade_id,
        ts_event,
        ts_init,
    )
}

#[must_use]
pub fn parse_bar_msg(
    msg: BarMsg,
    price_precision: u8,
    size_precision: u8,
    instrument_id: Option<InstrumentId>,
) -> Bar {
    let temp_exchange_str = serde_json::to_string(&msg.exchange)
        .unwrap()
        .trim_matches('"')
        .to_string();

    let instrument_id = match &instrument_id {
        Some(id) => *id,
        None => parse_instrument_id(&temp_exchange_str, &msg.symbol),
    };
    let spec = parse_bar_spec(&msg.name);
    let bar_type = BarType::new(instrument_id, spec, AggregationSource::External);

    let open = Price::new(msg.open, price_precision);
    let high = Price::new(msg.high, price_precision);
    let low = Price::new(msg.low, price_precision);
    let close = Price::new(msg.close, price_precision);
    let volume = Quantity::new(msg.volume, size_precision);
    let ts_event = UnixNanos::from(msg.timestamp.timestamp_nanos_opt().unwrap() as u64);
    let ts_init = UnixNanos::from(msg.local_timestamp.timestamp_nanos_opt().unwrap() as u64);

    Bar::new(bar_type, open, high, low, close, volume, ts_event, ts_init).unwrap()
}

////////////////////////////////////////////////////////////////////////////////
// Tests
////////////////////////////////////////////////////////////////////////////////
#[cfg(test)]
mod tests {
    use nautilus_model::enums::{AggressorSide, BookAction};
    use rstest::rstest;

    use super::*;
    use crate::tardis::tests::load_test_json;

    #[rstest]
    fn test_parse_book_change_message() {
        let json_data = load_test_json("book_change.json");
        let msg: BookChangeMsg = serde_json::from_str(&json_data).unwrap();

        let price_precision = 0;
        let size_precision = 0;
        let instrument_id = None;
        let deltas = parse_book_change_msg(msg, price_precision, size_precision, instrument_id);

        assert_eq!(deltas.deltas.len(), 1);
        assert_eq!(deltas.instrument_id, InstrumentId::from("XBTUSD.BITMEX"));
        assert_eq!(deltas.flags, RecordFlag::F_LAST.value());
        assert_eq!(deltas.sequence, 0);
        assert_eq!(deltas.ts_event, UnixNanos::from(1571830193469000000));
        assert_eq!(deltas.ts_init, UnixNanos::from(1571830193469000000));
        assert_eq!(
            deltas.deltas[0].instrument_id,
            InstrumentId::from("XBTUSD.BITMEX")
        );
        assert_eq!(deltas.deltas[0].action, BookAction::Update);
        assert_eq!(deltas.deltas[0].order.price, Price::from("7985"));
        assert_eq!(deltas.deltas[0].order.size, Quantity::from(283318));
        assert_eq!(deltas.deltas[0].order.order_id, 0);
        assert_eq!(deltas.deltas[0].flags, RecordFlag::F_LAST.value());
        assert_eq!(deltas.deltas[0].sequence, 0);
        assert_eq!(
            deltas.deltas[0].ts_event,
            UnixNanos::from(1571830193469000000)
        );
        assert_eq!(
            deltas.deltas[0].ts_init,
            UnixNanos::from(1571830193469000000)
        );
    }

    #[rstest]
    fn test_parse_book_snapshot_message() {
        let json_data = load_test_json("book_snapshot.json");
        let msg: BookSnapshotMsg = serde_json::from_str(&json_data).unwrap();

        let price_precision = 1;
        let size_precision = 0;
        let instrument_id = None;
        let deltas = parse_book_snapshot_msg(msg, price_precision, size_precision, instrument_id);
        let delta_0 = deltas.deltas[0];
        let _delta_2 = deltas.deltas[2];

        assert_eq!(deltas.deltas.len(), 4);
        assert_eq!(deltas.instrument_id, InstrumentId::from("XBTUSD.BITMEX"));
        assert_eq!(
            deltas.flags,
            RecordFlag::F_LAST.value() + RecordFlag::F_SNAPSHOT.value()
        );
        assert_eq!(deltas.sequence, 0);
        assert_eq!(deltas.ts_event, UnixNanos::from(1572010786950000000));
        assert_eq!(deltas.ts_init, UnixNanos::from(1572010786961000000));
        assert_eq!(delta_0.instrument_id, InstrumentId::from("XBTUSD.BITMEX"));
        assert_eq!(delta_0.action, BookAction::Add);
        assert_eq!(delta_0.order.price, Price::from("7633.5"));
        assert_eq!(delta_0.order.size, Quantity::from(1906067));
        assert_eq!(delta_0.order.order_id, 0);
        assert_eq!(delta_0.flags, RecordFlag::F_SNAPSHOT.value());
        assert_eq!(delta_0.sequence, 0);
        assert_eq!(delta_0.ts_event, UnixNanos::from(1572010786950000000));
        assert_eq!(delta_0.ts_init, UnixNanos::from(1572010786961000000));
        // TODO: Assert fields for delta_2 (top ask)
    }

    #[rstest]
    fn test_parse_trade_message() {
        let json_data = load_test_json("trade.json");
        let msg: TradeMsg = serde_json::from_str(&json_data).unwrap();

        let price_precision = 0;
        let size_precision = 0;
        let instrument_id = None;
        let trade = parse_trade_msg(msg, price_precision, size_precision, instrument_id);

        assert_eq!(trade.instrument_id, InstrumentId::from("XBTUSD.BITMEX"));
        assert_eq!(trade.price, Price::from("7996"));
        assert_eq!(trade.size, Quantity::from(50));
        assert_eq!(trade.aggressor_side, AggressorSide::Seller);
        assert_eq!(trade.ts_event, UnixNanos::from(1571826769669000000));
        assert_eq!(trade.ts_init, UnixNanos::from(1571826769740000000));
    }

    #[rstest]
    fn test_parse_bar_message() {
        let json_data = load_test_json("bar.json");
        let msg: BarMsg = serde_json::from_str(&json_data).unwrap();

        let price_precision = 1;
        let size_precision = 0;
        let instrument_id = None;
        let bar = parse_bar_msg(msg, price_precision, size_precision, instrument_id);

        assert_eq!(
            bar.bar_type,
            BarType::from("XBTUSD.BITMEX-10000-MILLISECOND-LAST-EXTERNAL")
        );
        assert_eq!(bar.open, Price::from("7623.5"));
        assert_eq!(bar.high, Price::from("7623.5"));
        assert_eq!(bar.low, Price::from("7623"));
        assert_eq!(bar.close, Price::from("7623.5"));
        assert_eq!(bar.volume, Quantity::from(37034));
        assert_eq!(bar.ts_event, UnixNanos::from(1572009100000000000));
        assert_eq!(bar.ts_init, UnixNanos::from(1572009100369000000));
    }
}
