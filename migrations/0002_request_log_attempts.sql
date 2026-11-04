-- T1.24 : nombre de tentatives par requête (détail du journal).
ALTER TABLE request_log ADD COLUMN attempts INTEGER NOT NULL DEFAULT 1;
