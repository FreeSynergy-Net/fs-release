//! Pattern: Message catalog (MF2 messages `blueprint-error-<code>` in en and de, one per code; ADR 0057; the variables are the parameters of the code).

use crate::error::Coded;

/// `(code, en, de)`: the message id is `blueprint-error-<code>`.
pub const MESSAGES: &[(&str, &str, &str)] = &[
    ("schema-unsupported", "Blueprint schema {$found} is not supported; this tool reads schema {$known} only", "Bauplan-Schema {$found} wird nicht unterstützt; dieses Werkzeug liest nur Schema {$known}"),
    ("schema-newer", "Blueprint schema {$found} is newer than the known schema {$known}", "Bauplan-Schema {$found} ist neuer als das bekannte Schema {$known}"),
    ("parse-error", "The blueprint cannot be read at line {$line}, column {$column}", "Der Bauplan ist in Zeile {$line}, Spalte {$column} nicht lesbar"),
    ("field-missing", "Table {$table} lacks the mandatory key {$field}", "Tabelle {$table} fehlt der Pflichtschlüssel {$field}"),
    ("field-empty", "Key {$field} of table {$table} needs at least one entry", "Schlüssel {$field} der Tabelle {$table} braucht mindestens einen Eintrag"),
    ("unknown-key", "Unknown key {$path}", "Unbekannter Schlüssel {$path}"),
    ("duplicate", "The name {$name} appears twice in table {$table}", "Der Name {$name} steht zweimal in Tabelle {$table}"),
    ("name-rule", "The program name {$name} breaks the naming rule (lower case, digits and hyphen, no fs- prefix)", "Der Programmname {$name} verstößt gegen die Namensregel (klein, Ziffern und Bindestrich, ohne Vorsatz fs-)"),
    ("name-case", "{$found} has the wrong letter case for {$key}", "{$found} hat für {$key} die falsche Schreibweise"),
    ("semver-invalid", "{$found} is no version major.minor.patch ({$key})", "{$found} ist keine Version Haupt.Neben.Patch ({$key})"),
    ("skeleton-unknown", "The skeleton version {$found} does not exist", "Die Gerüst-Version {$found} gibt es nicht"),
    ("skeleton-part-pending", "The skeleton part {$part} does not exist yet in this skeleton version", "Der Gerüst-Teil {$part} gibt es in dieser Gerüst-Version noch nicht"),
    ("capability-unknown", "The capability {$key} is not in the list of capabilities", "Die Fähigkeit {$key} steht nicht in der Liste der Fähigkeiten"),
    ("category-main-missing", "At least one main category of the desktop menu is needed", "Mindestens eine Hauptkategorie des Desktop-Menüs fehlt"),
    ("contract-version-invalid", "{$found} is no contract version major.minor", "{$found} ist keine Vertragsversion Haupt.Neben"),
    ("contract-id-invalid", "{$found} is no file id (0x and 16 hexadecimal digits)", "{$found} ist keine Datei-ID (0x und 16 Hexadezimalziffern)"),
    ("module-unknown", "The module {$name} is not in the catalog", "Das Modul {$name} steht nicht im Katalog"),
    ("bundle-unknown", "The bundle {$name} is not in the catalog", "Der Bausatz {$name} steht nicht im Katalog"),
    ("version-unknown", "{$module} has no tag {$version}", "{$module} hat keinen Tag {$version}"),
    ("kind-unknown", "The kind {$found} of module {$module} is unknown", "Die Art {$found} des Moduls {$module} ist unbekannt"),
    ("kind-mismatch", "Module {$module} is declared as {$declared} but the catalog says {$catalog}", "Modul {$module} ist als {$declared} angegeben, der Katalog sagt {$catalog}"),
    ("kind-wrong-role", "Module {$module} has the wrong kind for this use, expected {$expected}", "Modul {$module} hat für diese Verwendung die falsche Art, erwartet wird {$expected}"),
    ("bundle-part-unknown", "The bundle {$bundle} has no part {$part}", "Der Bausatz {$bundle} hat keinen Teil {$part}"),
    ("module-in-bundle", "{$module} is a part of bundle {$bundle} and may not be a module as well", "{$module} ist ein Teil des Bausatzes {$bundle} und darf nicht zusätzlich ein Modul sein"),
    ("type-unknown", "The type {$name} is not declared", "Der Typ {$name} ist nicht angegeben"),
    ("owned-by-module", "The type {$type} belongs to a module and carries no {$key} of its own", "Der Typ {$type} gehört einem Modul und hat kein eigenes {$key}"),
    ("standard-missing", "Field {$field} of type {$type} has no standard reference", "Feld {$field} des Typs {$type} hat keinen Standardbeleg"),
    ("standard-invalid", "{$found} is no standard address with a section (type {$type})", "{$found} ist keine Standard-Adresse mit Abschnitt (Typ {$type})"),
    ("media-type-invalid", "{$found} is no media type (type {$type})", "{$found} ist kein Medientyp (Typ {$type})"),
    ("field-kind-both", "Field {$field} names a kit and a basic type; exactly one is allowed", "Feld {$field} nennt Bausatz und Grundtyp; erlaubt ist genau eins"),
    ("field-kind-missing", "Field {$field} names neither a kit nor a basic type", "Feld {$field} nennt weder Bausatz noch Grundtyp"),
    ("kit-unknown", "The data kit {$name} does not exist", "Den Datenbausatz {$name} gibt es nicht"),
    ("basic-unknown", "The basic type {$name} does not exist", "Den Grundtyp {$name} gibt es nicht"),
    ("words-unknown", "The word list {$ref} does not exist or is not allowed here", "Die Wortliste {$ref} gibt es nicht oder sie ist hier nicht erlaubt"),
    ("ruleset-source-unknown", "The source {$name} of the rule set is no module or bundle of this blueprint", "Die Quelle {$name} des Regelwerks ist kein Modul und kein Bausatz dieses Bauplans"),
    ("ruleset-default-missing", "The default of rule set {$ruleset} is not one of its presets", "Die Voreinstellung des Regelwerks {$ruleset} steht nicht unter seinen Sätzen"),
    ("preset-unknown", "Rule set {$ruleset} names the unknown preset {$preset}", "Regelwerk {$ruleset} nennt den unbekannten Satz {$preset}"),
    ("layout-not-found", "The layout file {$path} does not exist", "Die Layout-Datei {$path} gibt es nicht"),
    ("layout-outside-forms", "The forms description {$path} must lie in the folder forms/ in the root of the repo, next to scaffold.toml", "Die forms-Beschreibung {$path} muss im Ordner forms/ im Wurzelordner des Repos liegen, neben scaffold.toml"),
    ("covers-invalid", "{$found} is not all and not an entry of the form Interface.method or Type.field", "{$found} ist weder all noch ein Eintrag der Form Schnittstelle.Methode oder Typ.Feld"),
    ("path-invalid", "{$path} is no valid repo path (relative, with slash, no .. and no wildcard)", "{$path} ist kein gültiger Repo-Pfad (relativ, mit Schrägstrich, ohne .. und ohne Platzhalter)"),
    ("path-not-found", "The path {$path} of the exception does not exist", "Der Pfad {$path} der Ausnahme existiert nicht"),
    ("path-generated", "The path {$path} covers a generated file", "Der Pfad {$path} deckt eine erzeugte Datei"),
    ("path-overlap", "The path {$path} is covered by exceptions {$first} and {$second}", "Der Pfad {$path} wird von den Ausnahmen {$first} und {$second} gedeckt"),
    ("file-unowned", "The file {$path} has no generator head and no exception", "Die Datei {$path} hat keinen Erzeuger-Kopf und keine Ausnahme"),
    ("main-reserved", "The path {$path} covers a main.rs: the entry point is generated and exchangeable, nothing program-specific may live there", "Der Pfad {$path} deckt eine main.rs: der Einstieg ist erzeugt und austauschbar, nichts Programmspezifisches darf dort stehen"),
    ("talk-not-found", "The talk record {$path} of the exception does not exist", "Das Gesprächsprotokoll {$path} der Ausnahme gibt es nicht"),
    ("envelope-invalid", "{$found} is neither a type of the ActivityStreams 2.0 vocabulary nor a full IRI (type {$type})", "{$found} ist weder ein Typ des ActivityStreams-2.0-Vokabulars noch eine volle IRI (Typ {$type})"),
    ("generated-changed", "The generated file {$path} differs from what the blueprint generates (changed by hand, or the blueprint changed)", "Die erzeugte Datei {$path} weicht von dem ab, was der Bauplan erzeugt (von Hand geändert oder der Bauplan hat sich geändert)"),
    ("generated-missing", "The generated file {$path} is missing", "Die erzeugte Datei {$path} fehlt"),
    ("seed-missing", "The seed file {$path} is missing", "Die Anlage-Datei {$path} fehlt"),
    ("slot-unknown", "The template names the unknown slot {$name}", "Die Vorlage nennt den unbekannten Platzhalter {$name}"),
    ("slot-open", "The template of {$path} has an open or unfilled slot", "Die Vorlage von {$path} hat einen offenen oder ungefüllten Platzhalter"),
    ("header-unsupported", "The file {$path} is of a kind that has no comment form for a head line", "Die Datei {$path} ist von einer Art, die keine Kommentarform für eine Kopfzeile hat"),
    ("artifact-duplicate", "Two emitters write the same path {$path}", "Zwei Erzeuger schreiben denselben Pfad {$path}"),
    ("exception-undeclared", "The path {$path} is marked as an exception but no exception of the blueprint names it", "Der Pfad {$path} ist als Ausnahme gekennzeichnet, aber keine Ausnahme des Bauplans nennt ihn"),
    ("write-failed", "The file {$path} could not be written", "Die Datei {$path} konnte nicht geschrieben werden"),
    ("file-unreadable", "The file {$path} is there but cannot be read as text; nothing was written", "Die Datei {$path} ist da, aber nicht als Text lesbar; es wurde nichts geschrieben"),
    ("path-symlink", "A part of the path {$path} is a symbolic link; nothing was written", "Ein Teil des Pfades {$path} ist ein symbolischer Link; es wurde nichts geschrieben"),
    ("line-ending", "The output for {$path} has a carriage return; generated files have LF only", "Die Ausgabe für {$path} hat ein Wagenrücklauf-Zeichen; erzeugte Dateien haben nur LF"),
    ("blueprint-unserializable", "The blueprint cannot be written in its canonical form", "Der Bauplan lässt sich nicht in seiner kanonischen Form schreiben"),
    ("text-pair", "The text {$key} lacks the language {$missing}", "Dem Text {$key} fehlt die Sprache {$missing}"),
    ("text-vars", "The text {$key} has different variables in en and de", "Der Text {$key} hat in en und de verschiedene Variablen"),
    ("text-syntax", "The text {$key} in {$lang} is not a simple MF2 message (plain text and variable placeholders only)", "Der Text {$key} in {$lang} ist keine einfache MF2-Nachricht (nur Text und Variablen-Platzhalter)"),
    ("fmt-changed", "The file {$path} is not in the canonical form", "Die Datei {$path} ist nicht in der kanonischen Form"),
    ("fmt-comments", "The file {$path} has comments that the canonical form cannot keep; nothing was written", "Die Datei {$path} hat Kommentare, die die kanonische Form nicht behalten kann; es wurde nichts geschrieben"),
];

/// The message of a code in `en` or `de`; `None` for an unknown code or language.
#[must_use]
pub fn message(code: &str, lang: &str) -> Option<&'static str> {
    let (_, en, de) = MESSAGES.iter().find(|(c, _, _)| *c == code)?;
    match lang {
        "en" => Some(en),
        "de" => Some(de),
        _ => None,
    }
}

/// The message with its variables filled from the parameters of the finding.
#[must_use]
pub fn render(c: &Coded, lang: &str) -> Option<String> {
    let mut text = message(c.code, lang)?.to_string();
    for (name, value) in &c.params {
        text = text.replace(&format!("{{${name}}}"), value);
    }
    Some(text)
}
