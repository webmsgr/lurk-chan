-- Add down migration script here
ALTER TABLE Reports add COLUMN audit text;
alter table Reports drop column location;