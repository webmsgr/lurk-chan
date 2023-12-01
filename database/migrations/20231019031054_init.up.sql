-- Add up migration script here
PRAGMA foreign_keys = on;
create table IF NOT EXISTS Reports  (
    id integer primary key,
    reporter_id text not null,
    reporter_name text not null,
    reported_id text not null,
    reported_name text not null,
    report_reason text not null,
    report_status text not null,
    server text not null,
    time text not null,
    claimant text,
    audit text
);

create index reporter on Reports (reporter_id);
create index reported on Reports (reported_id);
create index claimer on Reports (claimant);
create index reports_server on Reports (server);

create table if not exists Actions (
    id integer primary key not null,
    target_id text not null,
    target_username text not null,
    offense text not null,
    action text not null,
    server text not null,
    claimant text not null,
    report int,
    foreign key(report) references Reports(id)
);

create index targeted on Actions (target_id);
create index action_server on Actions (server);
