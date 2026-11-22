-- Run only on a disposable Presto instance with the memory catalog enabled.
-- Deliberately no IF NOT EXISTS: an existing table must fail rather than reuse data.
CREATE SCHEMA IF NOT EXISTS memory.dw;
CREATE TABLE memory.dw.sales_dtl AS
SELECT * FROM (VALUES ('web', 10), ('web', 20), ('store', 7))
    AS fixture(channel_cd, amount);
