# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0]
### Changed
- Use tokio v1, hyper v0.14, bytes v1

## [0.5.1]
### Added
- Decompression: lz4.

## [0.5.0]
### Added
- Decompression: gzip, zlib and brotli.

## [0.4.0]
### Added
- `Query::fetch_one()`, `Watch::fetch_one()`.
- `Query::fetch()` as a replacement for `Query::rows()`.
- `Watch::fetch()` as a replacement for `Watch::rows()`.
- `Watch::only_events().fetch()` as a replacement for `Watch::events()`.

### Changed
- `Error` is `StdError + Send + Sync + 'static` now.

## [0.3.0]
### Added
- Expose cursors (`query::RowCursor`, `watch::{RowCursor, EventCursor}`).

## [0.2.0]
### Added
- `Client::inserter()` for infinite inserting into tables.
- `Client::watch()` for `LIVE VIEW` related queries.

### Changed
- Renamed `Query::fetch()` to `Query::rows()`.
- Use `GET` requests for `SELECT` statements.

## [0.1.0]
### Added
- Support basic types.
- `Client::insert()` for inserting into tables.
- `Client::query()` for selecting from tables and DDL statements.
