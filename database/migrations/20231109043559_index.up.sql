-- Add up migration script here
create index if not exists action_claimer on Actions(claimant);