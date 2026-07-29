# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/SecurityRonin/whatsapp-desktop-forensic/releases/tag/whatsapp-desktop-core-v0.1.0) - 2026-07-29

### Added

- *(store)* GREEN — aggregate parse with dedup + deleted-record recovery
- *(crypto)* GREEN — AES-CBC body decryption (RustCrypto, fail-loud)
- *(timeline)* GREEN — time-ordered timeline + epoch->RFC3339
- *(contact)* GREEN — parse contact roster rows
- *(chat)* GREEN — parse chat roster rows
- *(media)* GREEN — extract media reference metadata from messages
- *(message)* GREEN — parse WhatsApp message metadata + encrypted body

### Documentation

- fix intra-doc links to the public decrypt_body re-export
