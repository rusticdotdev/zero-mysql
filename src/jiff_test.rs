use jiff::civil::{Date, DateTime, Time};
use zerocopy::FromBytes;

use crate::constant::ColumnType;
use crate::error::Result;
use crate::protocol::command::ColumnDefinitionTail;
use crate::protocol::r#trait::param::TypedParam;
use crate::raw::{FromRawValue, parse_value};
use crate::test_macros::{check_eq, check_err};

fn column(column_type: ColumnType) -> Result<ColumnDefinitionTail> {
    let mut bytes = [0u8; 12];
    bytes[6] = column_type as u8;
    Ok(*ColumnDefinitionTail::ref_from_bytes(&bytes)?)
}

fn encode<T: TypedParam>(value: T) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    value.encode_value(&mut out)?;
    Ok(out)
}

fn decode<'a, T: FromRawValue<'a>>(column_type: ColumnType, data: &'a [u8]) -> Result<T> {
    let (value, rest) = parse_value::<T>(&column(column_type)?, false, data)?;
    check_eq!(rest.len(), 0);
    Ok(value)
}

#[test]
fn date_round_trip() -> Result<()> {
    let date = Date::constant(2024, 2, 29);
    let bytes = encode(date)?;
    check_eq!(bytes, [4, 0xE8, 0x07, 2, 29]);
    check_eq!(decode::<Date>(ColumnType::MYSQL_TYPE_DATE, &bytes)?, date);
    Ok(())
}

#[test]
fn date_rejects_zero_and_out_of_range() -> Result<()> {
    check_err!(decode::<Date>(ColumnType::MYSQL_TYPE_DATE, &[0]));
    check_err!(decode::<Date>(
        ColumnType::MYSQL_TYPE_DATE,
        &[4, 0xE7, 0x07, 2, 29]
    ));
    check_err!(decode::<Date>(
        ColumnType::MYSQL_TYPE_DATE,
        &[4, 0x00, 0x00, 0, 0]
    ));
    // Year 40000 does not fit jiff's year range and must not wrap to a negative year.
    check_err!(decode::<Date>(
        ColumnType::MYSQL_TYPE_DATE,
        &[4, 0x40, 0x9C, 1, 1]
    ));
    Ok(())
}

#[test]
fn time_round_trip() -> Result<()> {
    let whole_seconds = Time::constant(23, 59, 58, 0);
    let bytes = encode(whole_seconds)?;
    check_eq!(bytes, [8, 0, 0, 0, 0, 0, 23, 59, 58]);
    check_eq!(
        decode::<Time>(ColumnType::MYSQL_TYPE_TIME, &bytes)?,
        whole_seconds
    );

    let with_micros = Time::constant(1, 2, 3, 123_456_000);
    let micros_bytes = encode(with_micros)?;
    check_eq!(
        micros_bytes,
        [12, 0, 0, 0, 0, 0, 1, 2, 3, 0x40, 0xE2, 0x01, 0x00]
    );
    check_eq!(
        decode::<Time>(ColumnType::MYSQL_TYPE_TIME, &micros_bytes)?,
        with_micros
    );

    check_eq!(
        decode::<Time>(ColumnType::MYSQL_TYPE_TIME, &[0])?,
        Time::midnight()
    );
    Ok(())
}

#[test]
fn time_truncates_below_microsecond() -> Result<()> {
    check_eq!(
        encode(Time::constant(1, 2, 3, 999))?,
        [8, 0, 0, 0, 0, 0, 1, 2, 3]
    );
    Ok(())
}

#[test]
fn time_rejects_negative_days_and_out_of_range() -> Result<()> {
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[8, 1, 0, 0, 0, 0, 1, 2, 3]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[8, 0, 1, 0, 0, 0, 1, 2, 3]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[12, 0, 1, 0, 0, 0, 1, 2, 3, 0, 0, 0, 0]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[8, 0, 0, 0, 0, 0, 24, 0, 0]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[8, 0, 0, 0, 0, 0, 0, 0, 60]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[12, 0, 0, 0, 0, 0, 1, 2, 3, 0x40, 0x42, 0x0F, 0x00]
    ));
    check_err!(decode::<Time>(
        ColumnType::MYSQL_TYPE_TIME,
        &[12, 0, 0, 0, 0, 0, 1, 2, 3, 0xFF, 0xFF, 0xFF, 0xFF]
    ));
    Ok(())
}

#[test]
fn datetime_round_trip() -> Result<()> {
    let whole_seconds = DateTime::constant(2024, 2, 29, 23, 59, 58, 0);
    let bytes = encode(whole_seconds)?;
    check_eq!(bytes, [7, 0xE8, 0x07, 2, 29, 23, 59, 58]);
    check_eq!(
        decode::<DateTime>(ColumnType::MYSQL_TYPE_DATETIME, &bytes)?,
        whole_seconds
    );

    let with_micros = DateTime::constant(2024, 2, 29, 1, 2, 3, 123_456_000);
    let micros_bytes = encode(with_micros)?;
    check_eq!(
        micros_bytes,
        [11, 0xE8, 0x07, 2, 29, 1, 2, 3, 0x40, 0xE2, 0x01, 0x00]
    );
    check_eq!(
        decode::<DateTime>(ColumnType::MYSQL_TYPE_TIMESTAMP, &micros_bytes)?,
        with_micros
    );

    check_eq!(
        decode::<DateTime>(ColumnType::MYSQL_TYPE_DATETIME, &[4, 0xE8, 0x07, 2, 29])?,
        DateTime::constant(2024, 2, 29, 0, 0, 0, 0)
    );
    Ok(())
}

#[test]
fn datetime_rejects_zero_and_out_of_range() -> Result<()> {
    check_err!(decode::<DateTime>(ColumnType::MYSQL_TYPE_DATETIME, &[0]));
    check_err!(decode::<DateTime>(
        ColumnType::MYSQL_TYPE_DATETIME,
        &[4, 0x00, 0x00, 0, 0]
    ));
    check_err!(decode::<DateTime>(
        ColumnType::MYSQL_TYPE_DATETIME,
        &[7, 0xE8, 0x07, 2, 30, 0, 0, 0]
    ));
    check_err!(decode::<DateTime>(
        ColumnType::MYSQL_TYPE_DATETIME,
        &[7, 0xE8, 0x07, 2, 29, 24, 0, 0]
    ));
    check_err!(decode::<DateTime>(
        ColumnType::MYSQL_TYPE_DATETIME,
        &[11, 0xE8, 0x07, 2, 29, 1, 2, 3, 0x40, 0x42, 0x0F, 0x00]
    ));
    Ok(())
}
