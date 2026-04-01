# Text processing for ContainsMatch (FTS5) and FulltextMatch (Tantivy) Search

The sutta texts are in HTML format, which we convert to plain text for ContainsMatch (Sqlite FTS5) and FulltextMatch (Tantivy) search.

The plain text version is stored in `suttas.content_plain` field, converted from HTML with `helpers.rs::sutta_html_to_plain_text()`.

Processing flow:

- `sutta_html_to_plain_text()` -- removes `<header>...</header>` to not include nikāya names in fulltext index
- `compact_rich_text()` -- strips html
- `compact_plain_text()` -- nomalizes spaces
- `consistent_niggahita()` -- ensures ṁ
- `normalize_iti_sandhi()` -- `mūlan'ti` → `mūlaṁ ti`
- `remove_punct()`
  - all punctuation to spaces
  - normalize spaces, newlines, tabs
  - Remove remaining straight quotes: `'` and `"`, `manopubbaṅ'gamā` → `manopubbaṅgamā`

- [ ] bootstrap

- ? When removing single and double quote marks, should we remove unicode smart quote?
Are there examples within compounds? `manopubbaṅ’gamā` (with smart quote)

`preprocess_text_for_word_extraction()`
- for glossing, dict matches
- `normalize_iti_sandhi()` -- `mūlan'ti` → `mūlaṁ ti`

- Should normalize to `word ti` or ``word nti`, separating the stem form
- stemmer will resolve `bhikkhūti` for fulltext
- contains match should match `bhikkhūti` when that is the query
- contains match should find `bhikkhūti` for the query `bhikkhu` or `bhikkhūti`
  - same for `bhikkhunti`, not the same problem as `-anti` verb endings

-----

Should match 'nti' variations and '-ati/-anti' verb endings.
Use fulltext stemmer to remove endings?

nibbānan”ti → nibbānaṁ ti

----

adhippeto bhikkhūti.

Stemmer should handle:
bhikkhūti → bhikkhu ti
bhikkhunti → bhikkhu ti

----

anabhijanam: no results

First page of results should include:

: pli-tv-bu-pm/pli/ms
: pli-tv-bu-vb-pj4/pli/ms
: Yo pana bhikkhu anabhijānaṁ uttarimanussadhammaṁ 

ContainsMatch 'anabhijānaṁ' - ok
Then switch to FulltextMatch - results ok, 'anabhijānaṁ uttarimanussadhammaṁ' shows
pli-tv-bu-vb-pj4/pli/ms but not pli-tv-bu-pm/pli/ms

'anabhijanam' FullText: no results
'anabhijanam uttarimanussadhammam' FullText: no results

----

anabhija
anabhijā -- ok results from both ms and cst, Pali stemmed from anabhijānaṁ

anabhijan
anabhijān -- no results, expected, not a valid stem for fulltext

anabhijana
anabhijāna -- results only from cst

vin01m.mul.xml/pli/cst includes:
: Yo pana bhikkhu anabhijānaṁ uttarimanussadhammaṁ
What matches from fulltext in search results:

Anabhijānanti asantaṁ → Anabhijāna nti asantaṁ

anabhijana + m: no results
anabhijāna + ṁ: results from ms and cst

-----

Typing anabhijānaṁ: first showing results with ms and cst suttas, but another query was launched and returns with cst only results

And old query overrides the results from a new query

-----

query:
nandi dukkhassa mula
nandi dukkhassa mulam

Should show
mn1/pli/ms
: ‘Nandī dukkhassa mūlan’ti
mn1/pli/cst
: ‘Nandī [nandi (sī. syā.)] dukkhassa mūla’nti
: ‘Nandī dukkhassa mūla’nti

mn1/pli/ms: nandi dukkhassa mulan ti iti -- doesn't match, 'mulan'

mn1/pli/cst: nandi dukkhassa mula nti iti -- matches


