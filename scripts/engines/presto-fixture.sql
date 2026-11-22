-- Run only on a disposable Presto instance with the memory catalog enabled.
-- Deliberately no IF NOT EXISTS: an existing table must fail rather than reuse data.
CREATE SCHEMA IF NOT EXISTS memory.dw;
CREATE TABLE memory.dw.sales_dtl AS
-- UTC timestamps straddle the inclusive Tokyo October range by one millisecond.
-- The zero-valued upper outsider preserves aggregate/filter totals but adds a
-- visible November bucket if the range incorrectly admits it.
SELECT * FROM (VALUES
    ('C1', 10, TIMESTAMP '2026-09-30 14:59:59.999'),
    ('C1', 20, TIMESTAMP '2026-09-30 15:00:00.000'),
    ('C2', 7, TIMESTAMP '2026-10-31 14:59:59.999'),
    ('C2', 0, TIMESTAMP '2026-10-31 15:00:00.000'))
    AS fixture(channel_cd, amount, sold_at);
