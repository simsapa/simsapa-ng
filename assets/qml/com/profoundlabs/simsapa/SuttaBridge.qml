import QtQuick

Item {
    id: root
    property bool db_loaded: false;
    property var dpd_lookup_test_data: ({})

    Component.onCompleted: {
        root.dpd_lookup_test_data = JSON.parse(dpd_lookup_data.json);
    }

    function load_db() {
        console.log("load_db()");
    }

    function appdata_first_query() {
        console.log("appdata_first_query()");
    }

    function dpd_first_query() {
        console.log("dpd_first_query()");
    }

    function results_page(query: string, page_num: int): string {
        console.log(query);
        return "{}";
    }

    function get_sutta_html(window_id: string, uid: string): string {
        var html = "<!doctype><html><body><h1>%1</h1></body></html>".arg(uid);
        return html;
    }

    function get_word_html(window_id: string, uid: string): string {
        var html = "<!doctype><html><body><h1>%1</h1></body></html>".arg(uid);
        return html;
    }

    function get_translations_for_sutta_uid(sutta_uid: string): list<string> {
        // See sutta_search_window_state.py _add_related_tabs()
        let uid_ref = sutta_uid.replace('^([^/]+)/.*', '$1');
        let translations = [
            `${uid_ref}/en/thanissaro`,
            `${uid_ref}/en/bodhi`,
            `${uid_ref}/en/sujato`,
        ];
        return translations;
    }

    function app_data_folder_path(): string {
        return "~/.local/share/simsapa-ng";
    }

    function is_app_data_folder_writable(): bool {
        return true;
    }

    function app_data_contents_html_table() {
        return `<table>
                    <tr>
                        <td>file</td>
                        <td>size</td>
                        <td>modified</td>
                    </tr>
                </table>`;
    }

    function app_data_contents_plain_table() {
        return `| file | size | modified |`;
    }

    function consistent_niggahita(text: string): string {
        if (text == null) {
            return "";
        }
        return text.replace("ṃ", "ṁ");
    }

    function normalize_query_text(text: string): string {
        text = consistent_niggahita(text);
        if (text.length == 0) {
            return text;
        }

        let normalizedText = text.toLowerCase();

        const reTi = /[''""]ti$/g;
        const reTrailPunct = /[\.,;:\!\?''"" ]+$/g;

        normalizedText = normalizedText.replace(reTi, "ti");
        normalizedText = normalizedText.replace(reTrailPunct, "");

        return normalizedText;
    }

    function dpd_deconstructor_list(query: string): list<string> {
        return [
            "olokita + saññāṇena + eva",
            "olokita + saññāṇena + iva",
        ];
    }

    function dpd_lookup_json(query: string): string {
        query = normalize_query_text(query);
        if (root.dpd_lookup_test_data[query]) {
            return JSON.stringify(root.dpd_lookup_test_data[query]);
        }
        return "[{}]";
    }

    function get_theme_name(): string {
        return 'dark';
    }

    function set_theme_name(theme_name: string) {
        console.log("set_theme_name():", theme_name);
    }

    function get_theme(theme_name: string): string {
        return '{}';
    }

    function get_saved_theme(): string {
        return '{}';
    }

    function get_common_words_json(): string {
        return `["a", "ānanda", "ariya", "bhagavā", "bhagavant", "bhikkhave", "bhikkhu", "bhikkhū", "ca", "cattāro", "ce", "dhamma", "dukkha", "dve", "eka", "eko", "etaṁ", "eva", "hi", "honti", "idaṁ", "idha", "kho", "magga", "na", "nirodha", "pana", "pañca", "pi", "sa", "samudaya", "sāriputta", "so", "ta", "taṁ", "tayo", "te", "tena", "ti", "va", "vā", "viharati", "yaṁ", "yo"]`;
    }

    function save_common_words_json(words_json: string) {
        return;
    }

    function get_gloss_history_json(): string {
        return "[]";
    }

    function update_gloss_session(session_uid: string, gloss_data_json: string) {
        return;
    }

    function save_new_gloss_session(gloss_data_json: string): string {
        return "session-123"; // saved db unique key
    }

    function save_anki_csv(csv_content: string): string {
        return "file_name.csv";
    }

    function save_file(folder_url: url, filename: string, content: string): bool {
        console.log(`save_file(): ${folder_url}, ${filename}, ${content}`);
        return true;
    }

    function check_file_exists_in_folder(folder_url: url, filename: string): bool {
        console.log(`check_file_exists_in_folder(): ${folder_url}, ${filename}`);
        return true;
    }

    Item {
        id: dpd_lookup_data
        visible: false
        property string json: `
{
  "akusalehi": [
    {
      "uid": "243/dpd",
      "word": "akusala 1",
      "summary": "<i>(adj)</i> (of a person or animal) unskilful; incompetent; inexperienced  <b>[na > a + kusala]</b>  <i>adj, from na kusala</i>"
    },
    {
      "uid": "244/dpd",
      "word": "akusala 2",
      "summary": "<i>(nt)</i> unbeneficial actions; unskilful deeds; the unwholesome  <b>[na > a + kusala]</b>  <i>nt, from na kusala</i>"
    },
    {
      "uid": "246/dpd",
      "word": "akusala 3",
      "summary": "<i>(adj)</i> unhealthy; unwholesome; unskilful; unbeneficial; kammically unprofitable  <b>[na > a + kusala]</b>  <i>adj, from na kusala</i>"
    }
  ],
  "ariyasāvakassa": [
    {
      "uid": "9022/dpd",
      "word": "ariyasāvaka",
      "summary": "<i>(masc)</i> disciple of the noble ones; a noble disciple  <b>[ariya + sāvaka]</b>  <i>masc, agent, comp</i>"
    }
  ],
  "ariyasāvako": [
    {
      "uid": "9022/dpd",
      "word": "ariyasāvaka",
      "summary": "<i>(masc)</i> disciple of the noble ones; a noble disciple  <b>[ariya + sāvaka]</b>  <i>masc, agent, comp</i>"
    }
  ],
  "bhikkhave": [
    {
      "uid": "49868/dpd",
      "word": "bhikkhave",
      "summary": "<i>(masc)</i> monks  <b>[√bhikkh + u + ave], [bhikkhu + ave]</b>  <i>masc, voc pl of bhikkhu</i>"
    },
    {
      "uid": "49885/dpd",
      "word": "bhikkhu",
      "summary": "<i>(masc)</i> monk; monastic; mendicant; fully ordained monk  <b>[√bhikkh + u]</b>  <i>masc, from bhikkhati</i>"
    }
  ],
  "cittassa": [
    {
      "uid": "26555/dpd",
      "word": "citta 1.1",
      "summary": "<i>(nt)</i> mind; heart  <b>[√cit + ta]</b>  <i>nt, from ceteti</i>"
    },
    {
      "uid": "26556/dpd",
      "word": "citta 1.2",
      "summary": "<i>(nt)</i> thought; intention; idea  <b>[√cit + ta]</b>  <i>nt, from ceteti</i>"
    },
    {
      "uid": "26563/dpd",
      "word": "citta 1.3",
      "summary": "<i>(nt)</i> thought moment, mental act  <b>[√cit + ta]</b>  <i>nt</i>"
    },
    {
      "uid": "26557/dpd",
      "word": "citta 2.1",
      "summary": "<i>(adj)</i> decorated; beautiful; adorned  <b>[√citt + a]</b>  <i>adj, from citteti</i>"
    },
    {
      "uid": "26558/dpd",
      "word": "citta 2.2",
      "summary": "<i>(adj)</i> varied; different; diverse  <b>[√citt + a]</b>  <i>adj, from citteti</i>"
    },
    {
      "uid": "26559/dpd",
      "word": "citta 2.3",
      "summary": "<i>(nt)</i> painting; picture; artwork; illusion  <b>[√citt + a]</b>  <i>nt, from citteti</i>"
    },
    {
      "uid": "26560/dpd",
      "word": "citta 2.4",
      "summary": "<i>(masc)</i> name of a householder lay-disciple; foremost lay disciple in giving Dhamma talks  <b>[√citt + a]</b>  <i>masc, from citteti</i>"
    },
    {
      "uid": "26561/dpd",
      "word": "citta 2.5",
      "summary": "<i>(masc)</i> name of a monk; Citta Hatthisāriputta  <b>[√citt + a]</b>  <i>masc, from citteti</i>"
    },
    {
      "uid": "26562/dpd",
      "word": "citta 2.6",
      "summary": "<i>(masc)</i> name of a lunar month; March-April  <b>[√citt + a]</b>  <i>masc, from citteti</i>"
    }
  ],
  "dhammehi": [
    {
      "uid": "34626/dpd",
      "word": "dhamma 1.01",
      "summary": "<i>(masc)</i> nature; character  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34627/dpd",
      "word": "dhamma 1.02",
      "summary": "<i>(masc)</i> quality; characteristic; trait; inherent quality  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34628/dpd",
      "word": "dhamma 1.03",
      "summary": "<i>(masc)</i> teaching; discourse; doctrine  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34629/dpd",
      "word": "dhamma 1.04",
      "summary": "<i>(masc)</i> mental phenomena; thoughts  <b>[√dhar + ma]</b>  <i>masc, normally pl dhammā, from dharati</i>"
    },
    {
      "uid": "34630/dpd",
      "word": "dhamma 1.05",
      "summary": "<i>(masc)</i> mental states  <b>[√dhar + ma]</b>  <i>masc, normally pl dhammā, from dharati</i>"
    },
    {
      "uid": "34631/dpd",
      "word": "dhamma 1.06",
      "summary": "<i>(masc)</i> matter; thing; phenomena  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34632/dpd",
      "word": "dhamma 1.07",
      "summary": "<i>(masc)</i> truth; reality; principle; truth behind the teaching  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34633/dpd",
      "word": "dhamma 1.08",
      "summary": "<i>(masc)</i> virtue; moral behaviour  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34634/dpd",
      "word": "dhamma 1.09",
      "summary": "<i>(masc)</i> law; case; rule; legal process  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34635/dpd",
      "word": "dhamma 1.10",
      "summary": "<i>(adj)</i> of such nature; liable (to); prone (to); destined (for)  <b>[√dhar + ma]</b>  <i>adj, from dharati</i>"
    },
    {
      "uid": "34636/dpd",
      "word": "dhamma 1.11",
      "summary": "<i>(masc)</i> name of king Mahāsudassana's palace  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34637/dpd",
      "word": "dhamma 1.12",
      "summary": "<i>(nt)</i> teaching; discourse; doctrine  <b>[√dhar + ma]</b>  <i>nt, from dharati, irreg</i>"
    },
    {
      "uid": "34638/dpd",
      "word": "dhamma 1.13",
      "summary": "<i>(masc)</i> religion  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34639/dpd",
      "word": "dhamma 1.14",
      "summary": "<i>(masc)</i> act; practice  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34640/dpd",
      "word": "dhamma 1.15",
      "summary": "<i>(masc)</i> duty; obligation  <b>[√dhar + ma]</b>  <i>masc, from dharati</i>"
    },
    {
      "uid": "34641/dpd",
      "word": "dhamma 2.1",
      "summary": "<i>(adj)</i> having a bow  <b>[dhanu + a > dhanva > dhamma]</b>  <i>adj, in comps, from dhanu</i>"
    }
  ],
  "ekaggataṁ": [
    {
      "uid": "17414/dpd",
      "word": "ekaggatā",
      "summary": "<i>(fem)</i> unification; oneness  <b>[eka + agga + tā], [ekagga + tā]</b>  <i>fem, abstr, comp, from ekagga</i>"
    }
  ],
  "etaṁ": [
    {
      "uid": "17896/dpd",
      "word": "etaṁ 1",
      "summary": "<i>(pron)</i> this; this thing (subject)  <b>[eta + aṁ]</b>  <i>pron, nt nom sg of eta</i>"
    },
    {
      "uid": "17897/dpd",
      "word": "etaṁ 2",
      "summary": "<i>(pron)</i> this; this man; this thing (object)  <b>[eta + aṁ]</b>  <i>pron, masc fem & nt acc sg of eta</i>"
    },
    {
      "uid": "17970/dpd",
      "word": "enta",
      "summary": "<i>(prp)</i> coming  <b>[e + nta]</b>  <i>prp of eti</i>"
    },
    {
      "uid": "17847/dpd",
      "word": "eta",
      "summary": "<i>(pron)</i> this   <i>pron, base</i>"
    },
    {
      "uid": "17914/dpd",
      "word": "eti 1",
      "summary": "<i>(pr)</i> comes (to)  <b>[e + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "17915/dpd",
      "word": "eti 2",
      "summary": "<i>(pr)</i> goes (to)  <b>[e + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "80558/dpd",
      "word": "eti 3",
      "summary": "<i>(pr)</i> becomes  <b>[e + ti]</b>  <i>pr</i>"
    }
  ],
  "hi": [
    {
      "uid": "71212/dpd",
      "word": "hi 1",
      "summary": "<i>(ind)</i> indeed; certainly; truly; definitely   <i>ind, emph</i>"
    },
    {
      "uid": "71213/dpd",
      "word": "hi 2",
      "summary": "<i>(ind)</i> because; for   <i>ind</i>"
    },
    {
      "uid": "71214/dpd",
      "word": "hi 3",
      "summary": "<i>(ve)</i> (gram) hi; verbal ending of imperative 2nd person singular   <i>ve, masc</i>"
    },
    {
      "uid": "√hi-1/dpd",
      "word": "√hi 1",
      "summary": "<b>√hi 1</b> send <b>·</b> <i>√hi 4 svādigaṇa + ṇā (send)</i>"
    },
    {
      "uid": "√hi-2/dpd",
      "word": "√hi 2",
      "summary": "<b>√hi 2</b> impel <b>·</b> <i>√hi×7 tanādigaṇa + o (impel)Base:hinoDhātupātha:hi gatiyaṁ (going) #525Dhātumañjūsa:hi gatimhi (going) #713Saddanīti:hi gati-buddhīsu upatāpe ca (going, knowing and vexation, tormenting)Sanskrit Root:√hi 1, 5 (impel)Pāṇinīya Dhātupāṭha:hi gatau vṛddhau ca (going and cutting off)</i>"
    }
  ],
  "hissa": [
    {
      "uid": "71304/dpd",
      "word": "hissa 1",
      "summary": "<i>(sandhi)</i> indeed his; certainly of that; truly his  <b>[hi + assa]</b>  <i>sandhi, ind + pron</i>"
    },
    {
      "uid": "71305/dpd",
      "word": "hissa 2",
      "summary": "<i>(sandhi)</i> certainly; truly; verily  <b>[hi + ssa]</b>  <i>sandhi, ind + pron</i>"
    },
    {
      "uid": "79690/dpd",
      "word": "hissa 3",
      "summary": "<i>(sandhi)</i> indeed for him; certainly to that; truly to him  <b>[hi + assa]</b>  <i>sandhi, ind + pron</i>"
    },
    {
      "uid": "71214/dpd",
      "word": "hi 3",
      "summary": "<i>(ve)</i> (gram) hi; verbal ending of imperative 2nd person singular   <i>ve, masc</i>"
    }
  ],
  "idha": [
    {
      "uid": "13686/dpd",
      "word": "idha 1",
      "summary": "<i>(ind)</i> here; now; in this world  <b>[ima + dha]</b>  <i>ind, adv, from ima</i>"
    },
    {
      "uid": "13687/dpd",
      "word": "idha 2",
      "summary": "<i>(ind)</i> here; in this regard; in this case  <b>[ima + dha]</b>  <i>ind, adv, from ima</i>"
    },
    {
      "uid": "13688/dpd",
      "word": "idha 3",
      "summary": "<i>(ind)</i> (comm) in this teaching; here in this doctrine  <b>[ima + dha]</b>  <i>ind, adv, from ima</i>"
    }
  ],
  "jhānaṁ": [
    {
      "uid": "28748/dpd",
      "word": "jhāna 1",
      "summary": "<i>(nt)</i> state of deep meditative calm  <b>[√jhā + ana]</b>  <i>nt, from jhāyati</i>"
    },
    {
      "uid": "28749/dpd",
      "word": "jhāna 2",
      "summary": "<i>(nt)</i> meditation; stage of meditation  <b>[√jhā + ana]</b>  <i>nt, from jhāyati</i>"
    },
    {
      "uid": "28750/dpd",
      "word": "jhāna 3",
      "summary": "<i>(adj)</i> having meditation; related to meditation  <b>[√jhā + ana]</b>  <i>adj, from jhāyati</i>"
    },
    {
      "uid": "80485/dpd",
      "word": "jhāna 4",
      "summary": "<i>(nt)</i> thinking about; contemplating  <b>[√jhā + ana]</b>  <i>nt, act, from jhāyati</i>"
    }
  ],
  "karitvā": [
    {
      "uid": "20502/dpd",
      "word": "karitvā 1",
      "summary": "<i>(abs)</i> having done; having performed  <b>[√kar + itvā]</b>  <i>abs of karoti</i>"
    },
    {
      "uid": "20503/dpd",
      "word": "karitvā 2",
      "summary": "<i>(abs)</i> having made  <b>[√kar + itvā]</b>  <i>abs of karoti</i>"
    },
    {
      "uid": "20504/dpd",
      "word": "karitvā 3",
      "summary": "<i>(abs)</i> having built; having constructed  <b>[√kar + itvā]</b>  <i>abs of karoti</i>"
    },
    {
      "uid": "80543/dpd",
      "word": "karitvā 4",
      "summary": "<i>(abs)</i> having compared (somebody with)  <b>[√kar + itvā]</b>  <i>abs of karoti</i>"
    }
  ],
  "katamañca": [
    {
      "uid": "19645/dpd",
      "word": "katama",
      "summary": "<i>(pron)</i> what?; which (of the many)?  <b>[ka + tama]</b>  <i>pron, interr, from ka</i>"
    }
  ],
  "kāmehi": [
    {
      "uid": "20957/dpd",
      "word": "kāma 1",
      "summary": "<i>(adj)</i> wishing (to); wanting (to); would be delighted (to)  <b>[√kam > kām + *a]</b>  <i>adj, in comps, from kāmeti</i>"
    },
    {
      "uid": "20958/dpd",
      "word": "kāma 2",
      "summary": "<i>(adj)</i> enjoying; fond (of); who likes; who loves  <b>[√kam > kām + *a]</b>  <i>adj, in comps, from kāmeti</i>"
    },
    {
      "uid": "20959/dpd",
      "word": "kāma 3",
      "summary": "<i>(masc)</i> sense desire (of); sensual pleasure (of)  <b>[√kam > kām + *a]</b>  <i>masc, from kāmeti</i>"
    },
    {
      "uid": "20960/dpd",
      "word": "kāma 4",
      "summary": "<i>(masc)</i> (objects of) pleasure; sensual pleasure; sexual pleasure  <b>[√kam > kām + *a]</b>  <i>masc, from kāmeti</i>"
    },
    {
      "uid": "21117/dpd",
      "word": "kāmeti",
      "summary": "<i>(pr)</i> desires; longs (for); is in love (with)  <b>[kāme + ti]</b>  <i>pr</i>"
    }
  ],
  "labhati": [
    {
      "uid": "55630/dpd",
      "word": "labhati 1",
      "summary": "<i>(pr)</i> gets; receives; obtains (something for)  <b>[labha + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "55631/dpd",
      "word": "labhati 2",
      "summary": "<i>(pr)</i> is possible; is permissible; is allowable  <b>[labha + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "78404/dpd",
      "word": "labhati 3",
      "summary": "<i>(pr)</i> gets through (to somebody); makes (somebody) understand  <b>[labha + ti]</b>  <i>pr</i>"
    }
  ],
  "labhissati": [
    {
      "uid": "55640/dpd",
      "word": "labhissati",
      "summary": "<i>(fut)</i> will get; will obtain  <b>[√labh + issa + ti]</b>  <i>fut of labhati</i>"
    },
    {
      "uid": "55630/dpd",
      "word": "labhati 1",
      "summary": "<i>(pr)</i> gets; receives; obtains (something for)  <b>[labha + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "55631/dpd",
      "word": "labhati 2",
      "summary": "<i>(pr)</i> is possible; is permissible; is allowable  <b>[labha + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "78404/dpd",
      "word": "labhati 3",
      "summary": "<i>(pr)</i> gets through (to somebody); makes (somebody) understand  <b>[labha + ti]</b>  <i>pr</i>"
    }
  ],
  "paṭhamaṁ": [
    {
      "uid": "41468/dpd",
      "word": "paṭhamaṁ 1",
      "summary": "<i>(ind)</i> first; firstly; at first; first of all  <b>[pa + √ṭhā + ma + aṁ], [paṭhama + aṁ]</b>  <i>ind, adv, acc sg of paṭhama</i>"
    },
    {
      "uid": "41469/dpd",
      "word": "paṭhamaṁ 2",
      "summary": "<i>(ind)</i> before; recently; newly; just  <b>[pa + √ṭhā + ma + aṁ], [paṭhama + aṁ]</b>  <i>ind, adv, acc sg of paṭhama</i>"
    },
    {
      "uid": "41150/dpd",
      "word": "paṭhama 1",
      "summary": "<i>(ordin)</i> first (1st); prime  <b>[pa + tama > ṭhama]</b>  <i>ordin, from pa</i>"
    },
    {
      "uid": "41151/dpd",
      "word": "paṭhama 2",
      "summary": "<i>(adj)</i> (gram) 3rd (person); he; she; it; they  <b>[pa + tama > ṭhama]</b>  <i>adj, from pa</i>"
    },
    {
      "uid": "41152/dpd",
      "word": "paṭhama 3",
      "summary": "<i>(masc)</i> (gram) first consonant of each vagga; k, c, ṭ, t, p  <b>[pa + tama > ṭhama]</b>  <i>masc, from pa</i>"
    },
    {
      "uid": "41470/dpd",
      "word": "paṭhamā",
      "summary": "<i>(fem)</i> (gram) nominative case   <i>fem</i>"
    }
  ],
  "pāṭikaṅkhaṁ": [
    {
      "uid": "45303/dpd",
      "word": "pāṭikaṅkha",
      "summary": "<i>(ptp)</i> to be expected (for); certain (for); can be anticipated  <b>[pati > pāṭi + √kaṅkh + *ya]</b>  <i>ptp of paṭikaṅkhati</i>"
    }
  ],
  "pītisukhaṁ": [
    {
      "uid": "46409/dpd",
      "word": "pītisukha 1",
      "summary": "<i>(nt)</i> delight and ease; joy and happiness  <b>[pīti + sukha]</b>  <i>nt, abstr, comp</i>"
    },
    {
      "uid": "75647/dpd",
      "word": "pītisukha 2",
      "summary": "<i>(adj)</i> having delight and ease; with joy and happiness  <b>[pīti + sukha]</b>  <i>adj, comp</i>"
    }
  ],
  "saddhassa": [
    {
      "uid": "58040/dpd",
      "word": "saddha 1",
      "summary": "<i>(adj)</i> faithful; confident; believing; devoted; trusting  <b>[sad + √dhā + ā + a], [saddhā + a]</b>  <i>adj, from saddhā</i>"
    },
    {
      "uid": "58041/dpd",
      "word": "saddha 2",
      "summary": "<i>(adj)</i> credulous; gullible  <b>[sad + √dhā + ā + a], [saddhā + a]</b>  <i>adj, from saddhā</i>"
    },
    {
      "uid": "58042/dpd",
      "word": "saddha 3",
      "summary": "<i>(masc)</i> name of a monk  <b>[sad + √dhā + ā + a], [saddhā + a]</b>  <i>masc, from saddhā</i>"
    },
    {
      "uid": "58043/dpd",
      "word": "saddha 4",
      "summary": "<i>(masc)</i> name of a monk; son of Sudatta  <b>[sad + √dhā + ā + a], [saddhā + a]</b>  <i>masc, from saddhā</i>"
    }
  ],
  "samādhi": [
    {
      "uid": "59623/dpd",
      "word": "samādhi 1",
      "summary": "<i>(masc)</i> perfect peace of mind; stability of mind; stillness of mind; mental composure  <b>[saṁ + ā + √dhā + i]</b>  <i>masc, abstr, from samādahati</i>"
    },
    {
      "uid": "59624/dpd",
      "word": "samādhi 2",
      "summary": "<i>(masc)</i> stability; stabilizer  <b>[saṁ + ā + √dhā + i]</b>  <i>masc, abstr, from samādahati</i>"
    }
  ],
  "samādhindriyaṁ": [
    {
      "uid": "59641/dpd",
      "word": "samādhindriya",
      "summary": "<i>(nt)</i> power of a collected mind; faculty of mental stability  <b>[samādhi + indriya]</b>  <i>nt, abstr, comp</i>"
    }
  ],
  "samādhiṁ": [
    {
      "uid": "59623/dpd",
      "word": "samādhi 1",
      "summary": "<i>(masc)</i> perfect peace of mind; stability of mind; stillness of mind; mental composure  <b>[saṁ + ā + √dhā + i]</b>  <i>masc, abstr, from samādahati</i>"
    },
    {
      "uid": "59624/dpd",
      "word": "samādhi 2",
      "summary": "<i>(masc)</i> stability; stabilizer  <b>[saṁ + ā + √dhā + i]</b>  <i>masc, abstr, from samādahati</i>"
    }
  ],
  "savicāraṁ": [
    {
      "uid": "61334/dpd",
      "word": "savicāra",
      "summary": "<i>(adj)</i> with management; with planning; with consideration  <b>[sa + vi + cāre + a]</b>  <i>adj, from vicāra</i>"
    }
  ],
  "savitakkaṁ": [
    {
      "uid": "61341/dpd",
      "word": "savitakka",
      "summary": "<i>(adj)</i> with thinking; accompanied with reflection  <b>[sa + vi + √takk + a]</b>  <i>adj, from vitakka</i>"
    }
  ],
  "so": [
    {
      "uid": "65082/dpd",
      "word": "so 1.1",
      "summary": "<i>(pron)</i> he; that person; that thing   <i>pron, masc nom sg of ta</i>"
    },
    {
      "uid": "65083/dpd",
      "word": "so 1.2",
      "summary": "<i>(ind)</i> (emphatic usage; referring to what has just been said)   <i>ind, emph</i>"
    },
    {
      "uid": "65084/dpd",
      "word": "so 2.1",
      "summary": "<i>(suffix)</i> as; according to; by way of; by means of   <i>suffix, adv, abl sg</i>"
    },
    {
      "uid": "65085/dpd",
      "word": "so 2.2",
      "summary": "<i>(suffix)</i> (gram) by; in x ways   <i>suffix, adv</i>"
    },
    {
      "uid": "56236/dpd",
      "word": "sa 1.1",
      "summary": "<i>(letter)</i> (gram) letter s; 36th letter of the alphabet; dental consonant   <i>letter, masc</i>"
    },
    {
      "uid": "56233/dpd",
      "word": "sa 3.1",
      "summary": "<i>(pron)</i> own; one's own; one's own possession   <i>pron, base, reflx</i>"
    },
    {
      "uid": "56234/dpd",
      "word": "sa 3.2",
      "summary": "<i>(adj)</i> self-; personal-   <i>adj, in comps, reflx</i>"
    },
    {
      "uid": "29188/dpd",
      "word": "ta 1.1",
      "summary": "<i>(pron)</i> that   <i>pron, base</i>"
    }
  ],
  "sāriputta": [
    {
      "uid": "62518/dpd",
      "word": "sāriputta",
      "summary": "<i>(masc)</i> name of an arahant monk; chief disciple; great disciple of the Buddha; foremost disciple in great wisdom  <b>[sāri + putta]</b>  <i>masc, matr, comp</i>"
    }
  ],
  "tadassa": [
    {
      "uid": "29794/dpd",
      "word": "tadassa 1",
      "summary": "<i>(sandhi)</i> that would be; that could be  <b>[tad + assa]</b>  <i>sandhi, pron + opt</i>"
    },
    {
      "uid": "29795/dpd",
      "word": "tadassa 2",
      "summary": "<i>(sandhi)</i> that is his  <b>[tad + assa]</b>  <i>sandhi, pron + pron</i>"
    }
  ],
  "upasampajja": [
    {
      "uid": "16124/dpd",
      "word": "upasampajja 1",
      "summary": "<i>(ger)</i> reaching; attaining; arriving at  <b>[upa + saṁ + √pad + ya]</b>  <i>ger of upasampajjati</i>"
    },
    {
      "uid": "16125/dpd",
      "word": "upasampajja 2",
      "summary": "<i>(ger)</i> becoming fully ordained; taking higher ordination  <b>[upa + saṁ + √pad + ya]</b>  <i>ger of upasampajjati</i>"
    },
    {
      "uid": "16126/dpd",
      "word": "upasampajja 3",
      "summary": "<i>(ger)</i> undertaking  <b>[upa + saṁ + √pad + ya]</b>  <i>ger of upasampajjati</i>"
    },
    {
      "uid": "16127/dpd",
      "word": "upasampajjati 1",
      "summary": "<i>(pr)</i> becomes fully ordained; takes higher ordination  <b>[upa + saṁ + pajja + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "80055/dpd",
      "word": "upasampajjati 2",
      "summary": "<i>(pr)</i> attains, enters  <b>[upa + saṁ + pajja + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "16128/dpd",
      "word": "upasampajji",
      "summary": "<i>(aor)</i> attained, entered on, became fully ordained  <b>[upa + saṁ + pajja +]</b>  <i>aor of upasampajjati</i>"
    }
  ],
  "upaṭṭhitassatino": [
    {
      "uid": "15578/dpd",
      "word": "upaṭṭhitassatī",
      "summary": "<i>(adj)</i> with presence of mind; attending mindfully  <b>[upaṭṭhita + satī]</b>  <i>adj, comp</i>"
    }
  ],
  "viharati": [
    {
      "uid": "69661/dpd",
      "word": "viharati 1",
      "summary": "<i>(pr)</i> lives (in); dwells (in); stays (in)  <b>[vi + hara + ti]</b>  <i>pr</i>"
    },
    {
      "uid": "69662/dpd",
      "word": "viharati 2",
      "summary": "<i>(pr)</i> stays (in); remains (in); continues (in); dwells (in)  <b>[vi + hara + ti]</b>  <i>pr</i>"
    }
  ],
  "vivekajaṁ": [
    {
      "uid": "69606/dpd",
      "word": "vivekaja",
      "summary": "<i>(adj)</i> born from seclusion; (or) born from discrimination; (comm) secluded from the defilements  <b>[viveka + ja]</b>  <i>adj, comp</i>"
    }
  ],
  "vivicca": [
    {
      "uid": "69591/dpd",
      "word": "vivicca",
      "summary": "<i>(ger)</i> separating (from); aloof (from)  <b>[vi + √vic + ya]</b>  <i>ger of viviccati</i>"
    },
    {
      "uid": "69592/dpd",
      "word": "viviccati",
      "summary": "<i>(pr)</i> is separate; is detached; is disengaged; is secluded (from)  <b>[vi + vicca + ti]</b>  <i>pr, pass of vi √vic</i>"
    }
  ],
  "vivicceva": [
    {
      "uid": "69593/dpd",
      "word": "vivicceva",
      "summary": "<i>(sandhi)</i> secluding oneself entirely (from)  <b>[vivicca + eva]</b>  <i>sandhi, ger + ind</i>"
    },
    {
      "uid": "69591/dpd",
      "word": "vivicca",
      "summary": "<i>(ger)</i> separating (from); aloof (from)  <b>[vi + √vic + ya]</b>  <i>ger of viviccati</i>"
    },
    {
      "uid": "69592/dpd",
      "word": "viviccati",
      "summary": "<i>(pr)</i> is separate; is detached; is disengaged; is secluded (from)  <b>[vi + vicca + ti]</b>  <i>pr, pass of vi √vic</i>"
    }
  ],
  "vossaggārammaṇaṁ": [
    {
      "uid": "70646/dpd",
      "word": "vossaggārammaṇa",
      "summary": "<i>(nt)</i> basis of letting go; foundation of complete relinquishment  <b>[vossagga + ārammaṇa]</b>  <i>nt, comp</i>"
    }
  ],
  "yaṁ": [
    {
      "uid": "53872/dpd",
      "word": "yaṁ 1",
      "summary": "<i>(pron)</i> which; whoever; whatever; that which  <b>[ya + aṁ]</b>  <i>pron, nt nom sg of ya</i>"
    },
    {
      "uid": "53873/dpd",
      "word": "yaṁ 2",
      "summary": "<i>(pron)</i> whoever; whatever; that which  <b>[ya + aṁ]</b>  <i>pron, masc fem & nt acc sg of ya</i>"
    },
    {
      "uid": "53874/dpd",
      "word": "yaṁ 3",
      "summary": "<i>(ind)</i> because; because of; since; when  <b>[ya + aṁ]</b>  <i>ind, adv, acc sg of ya</i>"
    },
    {
      "uid": "53445/dpd",
      "word": "ya 1.1",
      "summary": "<i>(letter)</i> (gram) letter y; 34th letter of the alphabet; palatal semi-vowel   <i>letter, masc</i>"
    },
    {
      "uid": "53444/dpd",
      "word": "ya 2.1",
      "summary": "<i>(pron)</i> whoever; whatever; whichever   <i>pron, base</i>"
    },
    {
      "uid": "53446/dpd",
      "word": "ya 3.1",
      "summary": "<i>(cs)</i> (gram) ya; suffix used to form impersonal and passive verbs   <i>cs, masc</i>"
    },
    {
      "uid": "53447/dpd",
      "word": "ya 3.2",
      "summary": "<i>(cs)</i> (gram) ya; conjugational sign of group 3 divādigaṇa verbs   <i>cs, masc</i>"
    }
  ],
  "yo": [
    {
      "uid": "54208/dpd",
      "word": "yo",
      "summary": "<i>(pron)</i> whoever; whatever; whichever   <i>pron, masc nom sg of ya</i>"
    },
    {
      "uid": "53445/dpd",
      "word": "ya 1.1",
      "summary": "<i>(letter)</i> (gram) letter y; 34th letter of the alphabet; palatal semi-vowel   <i>letter, masc</i>"
    },
    {
      "uid": "53444/dpd",
      "word": "ya 2.1",
      "summary": "<i>(pron)</i> whoever; whatever; whichever   <i>pron, base</i>"
    },
    {
      "uid": "53446/dpd",
      "word": "ya 3.1",
      "summary": "<i>(cs)</i> (gram) ya; suffix used to form impersonal and passive verbs   <i>cs, masc</i>"
    },
    {
      "uid": "53447/dpd",
      "word": "ya 3.2",
      "summary": "<i>(cs)</i> (gram) ya; conjugational sign of group 3 divādigaṇa verbs   <i>cs, masc</i>"
    }
  ],
  "āraddhavīriyassa": [
    {
      "uid": "12380/dpd",
      "word": "āraddhavīriya 1",
      "summary": "<i>(adj)</i> energetic (in); with energy aroused (to); applying energy (to); making an effort (to)  <b>[āraddha + vīriya]</b>  <i>adj, comp</i>"
    },
    {
      "uid": "74162/dpd",
      "word": "āraddhavīriya 2",
      "summary": "<i>(masc)</i> energetic person; who applies oneself; who makes an effort  <b>[āraddha + vīriya]</b>  <i>masc, comp</i>"
    }
  ]
}
`;
    }
}
