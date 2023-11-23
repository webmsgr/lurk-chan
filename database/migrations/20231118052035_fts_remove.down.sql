-- Add down migration script here
-- Add up migration script here
create virtual table ReportSearch using fts5(id unindexed, report_reason, tokenize = 'porter', content=Reports, content_rowid = id);
create virtual table ActionsSearch using fts5(id unindexed, offense, action, tokenize = 'porter', content=Actions, content_rowid = id);
insert into ReportSearch(id, report_reason) select id, report_reason from Reports;
insert into ActionsSearch(id, offense, action) select id, offense, action from Actions;

CREATE TRIGGER report_ai AFTER INSERT ON Reports BEGIN
    INSERT INTO ReportSearch(id, report_reason) VALUES (new.id, new.report_reason);
END;
CREATE TRIGGER report_ad before DELETE ON Reports BEGIN
    delete from ReportSearch where id = old.id;
END;
CREATE TRIGGER report_au AFTER UPDATE ON Reports BEGIN
    delete from ReportSearch where id = old.id;
    INSERT INTO ReportSearch(id, report_reason) VALUES (new.id, new.report_reason);
END;

CREATE TRIGGER actions_ai AFTER INSERT ON Actions BEGIN
    INSERT INTO ActionsSearch(id, offense, action) VALUES (new.id, new.offense, new.action);
END;
CREATE TRIGGER actions_ad before DELETE ON Actions BEGIN
    delete from ActionsSearch where id = old.id;
END;
CREATE TRIGGER actions_au AFTER UPDATE ON Actions BEGIN
    delete from ActionsSearch where id = old.id;
    INSERT INTO ActionsSearch(id, offense, action) VALUES (new.id, new.offense, new.action);
END;

insert into ReportSearch(ReportSearch) values('optimize');
insert into ActionsSearch(ActionsSearch) values('optimize');