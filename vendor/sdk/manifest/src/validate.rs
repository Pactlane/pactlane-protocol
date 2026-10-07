//! Semantic validation.
//!
//! Parsing proves a manifest is well-formed YAML in the right shape. This
//! module proves it describes a project that could actually run: the family is
//! implemented, kinds match the family, filters match their handler kind,
//! referenced files exist, start blocks are sane.
//!
//! Validation returns a [`ValidationReport`] rather than the first error, so
//! `superquery validate` can show every problem in one pass.

use camino::Utf8Path;
use miette::Diagnostic as MietteDiagnostic;
use std::collections::BTreeSet;
use superquery_types::{ChainFamily, ProjectId};
use thiserror::Error;

use crate::project::ProjectManifest;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// The project cannot run until this is fixed.
    Error,
    /// Legal, but very likely a mistake.
    Warning,
}

/// One validation finding, tied to a manifest field path.
#[derive(Debug, Clone, Error, MietteDiagnostic)]
#[error("{field}: {message}")]
#[diagnostic()]
pub struct Diagnostic {
    /// Dotted path to the offending field, e.g. `dataSources[0].handlers[1].kind`.
    pub field: String,
    /// What is wrong, in developer-facing language.
    pub message: String,
    /// How to fix it, when there is a concrete suggestion.
    #[help]
    pub help: Option<String>,
    /// Severity of this finding.
    pub severity: Severity,
}

impl Diagnostic {
    /// A blocking finding.
    fn error(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            help: None,
            severity: Severity::Error,
        }
    }

    /// A non-blocking finding.
    fn warning(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            help: None,
            severity: Severity::Warning,
        }
    }

    /// Attach a fix suggestion.
    fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// The result of validating one manifest.
#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    /// Every finding, in the order they were discovered.
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Findings that block the project from running.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }

    /// Non-blocking findings.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
    }

    /// Whether any blocking finding was recorded.
    pub fn has_errors(&self) -> bool {
        self.errors().next().is_some()
    }

    /// Turn a failing report into a [`crate::ManifestError`].
    pub fn into_result(self) -> Result<Self, crate::ManifestError> {
        if self.has_errors() {
            let errors: Vec<_> = self.errors().cloned().collect();
            return Err(crate::ManifestError::Invalid {
                count: errors.len(),
                diagnostics: errors,
            });
        }
        Ok(self)
    }

    fn push(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }
}

/// Validate a parsed manifest.
///
/// `base_dir` is the directory the manifest lives in; relative paths in the
/// manifest resolve against it. Pass `None` to skip all filesystem checks —
/// useful when validating a manifest that was fetched rather than read from
/// disk.
pub fn validate(manifest: &ProjectManifest, base_dir: Option<&Utf8Path>) -> ValidationReport {
    let mut report = ValidationReport::default();

    check_identity(manifest, &mut report);
    check_network(manifest, &mut report);
    check_schema(manifest, base_dir, &mut report);
    check_data_sources(manifest, base_dir, &mut report);

    report
}

fn check_identity(manifest: &ProjectManifest, report: &mut ValidationReport) {
    if let Err(err) = ProjectId::new(manifest.name.clone()) {
        report.push(Diagnostic::error("name", err.reason.clone()).with_help(
            "the project name becomes a directory, a database schema and a URL segment, \
                 so it is restricted to lowercase letters, digits and `-`",
        ));
    }

    if semver::Version::parse(&manifest.version).is_err() {
        report.push(
            Diagnostic::warning(
                "version",
                format!("`{}` is not a semver version", manifest.version),
            )
            .with_help("use a version like `0.1.0` so tooling can order releases"),
        );
    }
}

fn check_network(manifest: &ProjectManifest, report: &mut ValidationReport) {
    let network = &manifest.network;

    if !network.family.is_implemented() {
        report.push(
            Diagnostic::error(
                "network.family",
                format!(
                    "chain family `{}` is not implemented in this SDK build",
                    network.family
                ),
            )
            .with_help("only `evm` is supported today; Stellar and Solana land in Milestone 12"),
        );
    }

    if network.chain_id.trim().is_empty() {
        report.push(
            Diagnostic::error("network.chainId", "must not be empty")
                .with_help("for EVM this is the numeric chain id, e.g. \"1\" for Ethereum mainnet"),
        );
    } else if network.family == ChainFamily::Evm && network.chain_id.parse::<u64>().is_err() {
        report.push(
            Diagnostic::error(
                "network.chainId",
                format!("`{}` is not a numeric EVM chain id", network.chain_id),
            )
            .with_help("EVM chain ids are decimal numbers written as strings, e.g. \"1\""),
        );
    }

    if network.endpoint.is_empty() {
        report.push(
            Diagnostic::warning("network.endpoint", "no RPC endpoint declared")
                .with_help("the node will need one supplied at run time to index this project"),
        );
    }
}

fn check_schema(
    manifest: &ProjectManifest,
    base_dir: Option<&Utf8Path>,
    report: &mut ValidationReport,
) {
    let Some(base) = base_dir else { return };
    let path = base.join(&manifest.schema.file);
    if !path.is_file() {
        report.push(
            Diagnostic::error(
                "schema.file",
                format!("`{}` does not exist", manifest.schema.file),
            )
            .with_help("create the GraphQL entity schema, or fix the path"),
        );
    }
}

fn check_data_sources(
    manifest: &ProjectManifest,
    base_dir: Option<&Utf8Path>,
    report: &mut ValidationReport,
) {
    if manifest.data_sources.is_empty() {
        report.push(
            Diagnostic::error(
                "dataSources",
                "a project must declare at least one datasource",
            )
            .with_help("add an entry describing the contract or blocks to index"),
        );
        return;
    }

    let family = manifest.network.family;
    let mut seen_names = BTreeSet::new();
    let mut seen_handlers = BTreeSet::new();

    for (i, ds) in manifest.data_sources.iter().enumerate() {
        let at = |suffix: &str| format!("dataSources[{i}]{suffix}");

        if let Some(name) = &ds.name
            && !seen_names.insert(name.clone())
        {
            report.push(
                Diagnostic::error(at(".name"), format!("duplicate datasource name `{name}`"))
                    .with_help("datasource names must be unique within a project"),
            );
        }

        if !ds.kind.belongs_to(family) {
            report.push(
                Diagnostic::error(
                    at(".kind"),
                    format!("`{}` is not a `{family}` datasource kind", ds.kind),
                )
                .with_help(format!(
                    "expected a kind prefixed `{}/`",
                    family.kind_prefix()
                )),
            );
        }

        let start = manifest.effective_start_block(ds);
        if let Some(end) = ds.end_block
            && end < start
        {
            report.push(Diagnostic::error(
                at(".endBlock"),
                format!("endBlock {end} is before startBlock {start}"),
            ));
        }

        for (name, asset) in &ds.assets {
            let Some(base) = base_dir else { continue };
            if !base.join(&asset.file).is_file() {
                report.push(
                    Diagnostic::error(
                        at(&format!(".assets.{name}.file")),
                        format!("`{}` does not exist", asset.file),
                    )
                    .with_help("check the path, or run `superquery codegen` after adding the file"),
                );
            }
        }

        if ds.handlers.is_empty() {
            report.push(
                Diagnostic::error(
                    at(".handlers"),
                    "a datasource must declare at least one handler",
                )
                .with_help("without a handler this datasource would index nothing"),
            );
        }

        for (j, handler) in ds.handlers.iter().enumerate() {
            let at_h = |suffix: &str| format!("dataSources[{i}].handlers[{j}]{suffix}");

            if handler.handler.trim().is_empty() {
                report.push(Diagnostic::error(
                    at_h(".handler"),
                    "handler name must not be empty",
                ));
            } else if !seen_handlers.insert(handler.handler.clone()) {
                report.push(
                    Diagnostic::warning(
                        at_h(".handler"),
                        format!(
                            "`{}` is referenced by more than one handler entry",
                            handler.handler
                        ),
                    )
                    .with_help("this is legal, but usually means a copy/paste slip"),
                );
            }

            if !handler.kind.belongs_to(family) {
                report.push(
                    Diagnostic::error(
                        at_h(".kind"),
                        format!("`{}` is not a `{family}` handler kind", handler.kind),
                    )
                    .with_help(format!(
                        "expected a kind prefixed `{}/`",
                        family.kind_prefix()
                    )),
                );
                continue;
            }

            check_filter_shape(handler, &at_h(".filter"), report);
        }
    }
}

/// A filter must match the shape its handler kind expects. `evm/LogHandler`
/// with a `function:` filter would silently never fire, so it is an error.
fn check_filter_shape(
    handler: &crate::handler::Handler,
    field: &str,
    report: &mut ValidationReport,
) {
    use crate::filter::HandlerFilter;

    let Some(filter) = &handler.filter else {
        return;
    };
    let Some(suffix) = handler.kind.suffix() else {
        return;
    };

    let expected = match suffix {
        s if s.ends_with("LogHandler") || s.ends_with("EventHandler") => "log",
        s if s.ends_with("TransactionHandler") || s.ends_with("CallHandler") => "transaction",
        s if s.ends_with("BlockHandler") => "block",
        // Unknown kinds are reported by the family check; nothing to add here.
        _ => return,
    };

    let actual = match filter {
        HandlerFilter::Log(_) => "log",
        HandlerFilter::Transaction(_) => "transaction",
        HandlerFilter::Block(_) => "block",
    };

    if expected != actual {
        report.push(
            Diagnostic::error(
                field.to_owned(),
                format!(
                    "handler kind `{}` expects a {expected} filter, but found a {}",
                    handler.kind,
                    filter.shape()
                ),
            )
            .with_help("a mismatched filter never matches, so the handler would never run"),
        );
    }
}
