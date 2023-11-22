-- Add up migration script here
create table if not exists AuditEdits (
    action_id integer not null,
    old text not null,
    new text not null,
    who text not null,
    time text not null,
    changes text not null,
    foreign key(action_id) references Actions(id)
);

create index AuditEdits_action_id on AuditEdits(action_id);
create index AuditEdits_who on AuditEdits(who);