CREATE DATABASE dw;
CREATE TABLE dw.sales (region VARCHAR(64), amount DOUBLE) DUPLICATE KEY(region)
DISTRIBUTED BY HASH(region) BUCKETS 1 PROPERTIES ('replication_num'='1');
INSERT INTO dw.sales VALUES ('north',10),('north',20),('a%_\\b',5),
('axxb',99),('north_central%',7),('north_west',11),('south_west',100);
CREATE TABLE dw.sales_dtl (store_cd VARCHAR(16), amount DOUBLE) DUPLICATE KEY(store_cd)
DISTRIBUTED BY HASH(store_cd) BUCKETS 1 PROPERTIES ('replication_num'='1');
CREATE TABLE dw.dim_store (store_cd VARCHAR(16), store_name VARCHAR(32), region_cd VARCHAR(16))
DUPLICATE KEY(store_cd) DISTRIBUTED BY HASH(store_cd) BUCKETS 1 PROPERTIES ('replication_num'='1');
CREATE TABLE dw.dim_region (region_cd VARCHAR(16), region_name VARCHAR(32)) DUPLICATE KEY(region_cd)
DISTRIBUTED BY HASH(region_cd) BUCKETS 1 PROPERTIES ('replication_num'='1');
INSERT INTO dw.sales_dtl VALUES ('s1',10),('s1',20),('s2',99);
INSERT INTO dw.dim_store VALUES ('s1','central','r1'),('s2','central','r2');
INSERT INTO dw.dim_region VALUES ('r1','north'),('r2','south');
