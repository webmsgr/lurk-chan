select R.id as id,
       reporter_id,
       reporter_name,
       reported_id,
       reported_name,
       highlight(ReportSearch, 1, '*', '*') as "report_reason: String",
       report_status as "report_status: ReportStatus",
       server,
       time,
       claimant,
       audit
from ReportSearch
         join Reports R on ReportSearch.id = R.id
where ReportSearch.report_reason match ?
order by bm25(ReportSearch)
limit 10;
