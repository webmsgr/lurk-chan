for 1.2.0



* ~~display number of past reports in report message~~
* ~~refactor /past to only pull 5 rows from the DB instead of them all. (using count(*) to get the count) (god why did i query them all)~~
* ~~change it so only the claim button shows up if open, and the close and close without action button only work for those who claimed it~~
* ~~refactor the audit log modal to use a AuditModalAutofill object for autofilling, then create From<Report> and From<Audit> impls. ~~
  * ~~the audit_modal() fn would take Option<AuditModalAutofill> for autofilling~~
* ~~update /audit to have optional arguments, each one coresponding to a field. then we can autofill (based)~~
* ~~Responds to pings with messages~~
* ~~commands dont autoregister~~
* ~~add "/judgement day" :)~~
* ~~switch to anyhow and pretty much completely redo error handling (pain and suffering)~~


for 1.2.1
* maybe: right click lurkbot embeds -> apps -> fill_audit
* add console commands to query db all under "db" command. "db top claims" "db top reported", etc etc. just go fuckign wild
* maybe: allow users in main to report people with the bot (#discord-reports) (this will be abused, 100%)
* refine the permission system
* maybe: somehow autoconvert normal audit log messages (cringe) to the cool lurk chan ones (based) (good fucking luck chief) (if they fucking follow the format)
* /open to see open reports (for admins looking for things to do)
* changelog