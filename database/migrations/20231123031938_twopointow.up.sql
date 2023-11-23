-- Add up migration script here
ALTER TABLE Reports add COLUMN location text not null default 'sl';
alter table Reports drop column audit;