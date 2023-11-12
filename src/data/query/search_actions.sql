select A.id as id,
       target_id,
       target_username,
       highlight(ActionsSearch, 1, '*', '*') as "offense: String",
       highlight(ActionsSearch, 2, '*', '*') as "action: String",
       server as "server: Location",
       claimant,
       report
from ActionsSearch
         join Actions A on ActionsSearch.id = A.id
where ActionsSearch match ?
order by bm25(ActionsSearch)
limit 10;