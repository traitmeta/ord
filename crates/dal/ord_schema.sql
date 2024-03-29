CREATE TABLE `ord_inscription_entries` (
    `id` varchar(256) NOT NULL COMMENT 'inscription id',
    `charms` INT NOT NULL DEFAULT '0' COMMENT 'charms',
    `fee` BIGINT(20) NOT NULL DEFAULT '0' COMMENT 'fee',
    `height` BIGINT(20)  NOT NULL DEFAULT '0' COMMENT 'height',
    `inscription_number` BIGINT(20)  COMMENT 'inscription_number',
    `parent` BIGINT(20)    DEFAULT NULL COMMENT 'parent number',
    `sat` BIGINT(20)  DEFAULT NULL COMMENT 'sat',
    `sequence_number` BIGINT(20)    DEFAULT NULL COMMENT 'sequence_number',
    `timestamp` BIGINT(20) NOT NULL DEFAULT '0' COMMENT 'timestamp',
    `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'create time',
    `updated_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT 'update time',
    PRIMARY KEY (`id`),
    UNIQUE KEY `uniq_sequence_number` (`sequence_number`),
    KEY `idx_inscription_number` (`inscription_number`)
) ENGINE = InnoDB DEFAULT CHARSET = utf8 COMMENT = 'inscription entries';

CREATE TABLE `ord_transactions` (
    `id` bigint(20) unsigned NOT NULL AUTO_INCREMENT COMMENT 'primary_key',
    `hash` varchar(256) NOT NULL DEFAULT '' COMMENT 'transaction hash',
    `raw_data` varchar(256) NOT NULL DEFAULT '' COMMENT 'raw tx data',
    `created_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP COMMENT 'create time',
    `updated_at` timestamp NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP COMMENT 'update time',
    PRIMARY KEY (`id`),
    UNIQUE KEY `uniq_hash` (`hash`)
) ENGINE = InnoDB DEFAULT CHARSET = utf8 COMMENT = 'ord transaction with inscription';

