-- Add up migration script here
create table if not exists ReportMessages (
    report_id integer unique not null,
    message text unique not null,
    channel text not null,
    foreign key(report_id) references Reports(id)
);


create table if not exists ActionMessages (
    action_id integer unique not null,
    message text unique not null,
    channel text not null,
    foreign key(action_id) references Actions(id)
)