/**
 * The versions of the CS2KZ plugin are also stored in the database.
 * This is mainly for validating servers as git already keeps track of versioning.
 *
 * The `version` column refers to the plugin's semver version.
 * The `revision` column refers to the plugin's git revision hash.
 */
CREATE TABLE IF NOT EXISTS PluginVersions (
  `id` INT2 UNSIGNED NOT NULL AUTO_INCREMENT,
  `version` VARCHAR(14) NOT NULL,
  `revision` VARCHAR(255) NOT NULL,
  `created_on` TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (`id`),
  UNIQUE (`version`),
  UNIQUE (`revision`)
);
