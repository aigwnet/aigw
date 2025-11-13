use std::{collections::HashMap, time::Duration};

use lazy_static::lazy_static;
use rbatis::{PageRequest, RBatis, rbdc::DateTime};
use serde::{Deserialize, Serialize};

use crate::{
    service::{map::LinkedHashMap, se_task::Task},
    storage::{
        tb_analytics_traffic::TbAnalyticsTraffic,
        tb_analytics_traffic_cluster::TbAnalyticsTrafficCluster,
        tb_analytics_traffic_cluster_hour::TbAnalyticsTrafficClusterHour,
    },
};

const ISO_3116_STR: &str = r#"
[{"alpha2":"AF","name":"Afghanistan"},{"alpha2":"AX","name":"Åland Islands"},{"alpha2":"AL","name":"Albania"},{"alpha2":"DZ","name":"Algeria"},{"alpha2":"AS","name":"American Samoa"},{"alpha2":"AD","name":"Andorra"},{"alpha2":"AO","name":"Angola"},{"alpha2":"AI","name":"Anguilla"},{"alpha2":"AQ","name":"Antarctica"},{"alpha2":"AG","name":"Antigua and Barbuda"},{"alpha2":"AR","name":"Argentina"},{"alpha2":"AM","name":"Armenia"},{"alpha2":"AW","name":"Aruba"},{"alpha2":"AU","name":"Australia"},{"alpha2":"AT","name":"Austria"},{"alpha2":"AZ","name":"Azerbaijan"},{"alpha2":"BS","name":"Bahamas"},{"alpha2":"BH","name":"Bahrain"},{"alpha2":"BD","name":"Bangladesh"},{"alpha2":"BB","name":"Barbados"},{"alpha2":"BY","name":"Belarus"},{"alpha2":"BE","name":"Belgium"},{"alpha2":"BZ","name":"Belize"},{"alpha2":"BJ","name":"Benin"},{"alpha2":"BM","name":"Bermuda"},{"alpha2":"BT","name":"Bhutan"},{"alpha2":"BO","name":"Bolivia (Plurinational State of)"},{"alpha2":"BQ","name":"Bonaire, Sint Eustatius and Saba"},{"alpha2":"BA","name":"Bosnia and Herzegovina"},{"alpha2":"BW","name":"Botswana"},{"alpha2":"BV","name":"Bouvet Island"},{"alpha2":"BR","name":"Brazil"},{"alpha2":"IO","name":"British Indian Ocean Territory"},{"alpha2":"BN","name":"Brunei Darussalam"},{"alpha2":"BG","name":"Bulgaria"},{"alpha2":"BF","name":"Burkina Faso"},{"alpha2":"BI","name":"Burundi"},{"alpha2":"CV","name":"Cabo Verde"},{"alpha2":"KH","name":"Cambodia"},{"alpha2":"CM","name":"Cameroon"},{"alpha2":"CA","name":"Canada"},{"alpha2":"KY","name":"Cayman Islands"},{"alpha2":"CF","name":"Central African Republic"},{"alpha2":"TD","name":"Chad"},{"alpha2":"CL","name":"Chile"},{"alpha2":"CN","name":"China"},{"alpha2":"CX","name":"Christmas Island"},{"alpha2":"CC","name":"Cocos (Keeling) Islands"},{"alpha2":"CO","name":"Colombia"},{"alpha2":"KM","name":"Comoros"},{"alpha2":"CG","name":"Congo"},{"alpha2":"CD","name":"Congo (Democratic Republic of the)"},{"alpha2":"CK","name":"Cook Islands"},{"alpha2":"CR","name":"Costa Rica"},{"alpha2":"CI","name":"Côte d'Ivoire"},{"alpha2":"HR","name":"Croatia"},{"alpha2":"CU","name":"Cuba"},{"alpha2":"CW","name":"Curaçao"},{"alpha2":"CY","name":"Cyprus"},{"alpha2":"CZ","name":"Czechia"},{"alpha2":"DK","name":"Denmark"},{"alpha2":"DJ","name":"Djibouti"},{"alpha2":"DM","name":"Dominica"},{"alpha2":"DO","name":"Dominican Republic"},{"alpha2":"EC","name":"Ecuador"},{"alpha2":"EG","name":"Egypt"},{"alpha2":"SV","name":"El Salvador"},{"alpha2":"GQ","name":"Equatorial Guinea"},{"alpha2":"ER","name":"Eritrea"},{"alpha2":"EE","name":"Estonia"},{"alpha2":"ET","name":"Ethiopia"},{"alpha2":"FK","name":"Falkland Islands (Malvinas)"},{"alpha2":"FO","name":"Faroe Islands"},{"alpha2":"FJ","name":"Fiji"},{"alpha2":"FI","name":"Finland"},{"alpha2":"FR","name":"France"},{"alpha2":"GF","name":"French Guiana"},{"alpha2":"PF","name":"French Polynesia"},{"alpha2":"TF","name":"French Southern Territories"},{"alpha2":"GA","name":"Gabon"},{"alpha2":"GM","name":"Gambia"},{"alpha2":"GE","name":"Georgia"},{"alpha2":"DE","name":"Germany"},{"alpha2":"GH","name":"Ghana"},{"alpha2":"GI","name":"Gibraltar"},{"alpha2":"GR","name":"Greece"},{"alpha2":"GL","name":"Greenland"},{"alpha2":"GD","name":"Grenada"},{"alpha2":"GP","name":"Guadeloupe"},{"alpha2":"GU","name":"Guam"},{"alpha2":"GT","name":"Guatemala"},{"alpha2":"GG","name":"Guernsey"},{"alpha2":"GN","name":"Guinea"},{"alpha2":"GW","name":"Guinea-Bissau"},{"alpha2":"GY","name":"Guyana"},{"alpha2":"HT","name":"Haiti"},{"alpha2":"HM","name":"Heard Island and McDonald Islands"},{"alpha2":"VA","name":"Holy See"},{"alpha2":"HN","name":"Honduras"},{"alpha2":"HK","name":"Hong Kong"},{"alpha2":"HU","name":"Hungary"},{"alpha2":"IS","name":"Iceland"},{"alpha2":"IN","name":"India"},{"alpha2":"ID","name":"Indonesia"},{"alpha2":"IR","name":"Iran (Islamic Republic of)"},{"alpha2":"IQ","name":"Iraq"},{"alpha2":"IE","name":"Ireland"},{"alpha2":"IM","name":"Isle of Man"},{"alpha2":"IL","name":"Israel"},{"alpha2":"IT","name":"Italy"},{"alpha2":"JM","name":"Jamaica"},{"alpha2":"JP","name":"Japan"},{"alpha2":"JE","name":"Jersey"},{"alpha2":"JO","name":"Jordan"},{"alpha2":"KZ","name":"Kazakhstan"},{"alpha2":"KE","name":"Kenya"},{"alpha2":"KI","name":"Kiribati"},{"alpha2":"KP","name":"Korea (Democratic People's Republic of)"},{"alpha2":"KR","name":"Korea (Republic of)"},{"alpha2":"KW","name":"Kuwait"},{"alpha2":"KG","name":"Kyrgyzstan"},{"alpha2":"LA","name":"Lao People's Democratic Republic"},{"alpha2":"LV","name":"Latvia"},{"alpha2":"LB","name":"Lebanon"},{"alpha2":"LS","name":"Lesotho"},{"alpha2":"LR","name":"Liberia"},{"alpha2":"LY","name":"Libya"},{"alpha2":"LI","name":"Liechtenstein"},{"alpha2":"LT","name":"Lithuania"},{"alpha2":"LU","name":"Luxembourg"},{"alpha2":"MO","name":"Macao"},{"alpha2":"MK","name":"North Macedonia"},{"alpha2":"MG","name":"Madagascar"},{"alpha2":"MW","name":"Malawi"},{"alpha2":"MY","name":"Malaysia"},{"alpha2":"MV","name":"Maldives"},{"alpha2":"ML","name":"Mali"},{"alpha2":"MT","name":"Malta"},{"alpha2":"MH","name":"Marshall Islands"},{"alpha2":"MQ","name":"Martinique"},{"alpha2":"MR","name":"Mauritania"},{"alpha2":"MU","name":"Mauritius"},{"alpha2":"YT","name":"Mayotte"},{"alpha2":"MX","name":"Mexico"},{"alpha2":"FM","name":"Micronesia (Federated States of)"},{"alpha2":"MD","name":"Moldova (Republic of)"},{"alpha2":"MC","name":"Monaco"},{"alpha2":"MN","name":"Mongolia"},{"alpha2":"ME","name":"Montenegro"},{"alpha2":"MS","name":"Montserrat"},{"alpha2":"MA","name":"Morocco"},{"alpha2":"MZ","name":"Mozambique"},{"alpha2":"MM","name":"Myanmar"},{"alpha2":"NA","name":"Namibia"},{"alpha2":"NR","name":"Nauru"},{"alpha2":"NP","name":"Nepal"},{"alpha2":"NL","name":"Netherlands"},{"alpha2":"NC","name":"New Caledonia"},{"alpha2":"NZ","name":"New Zealand"},{"alpha2":"NI","name":"Nicaragua"},{"alpha2":"NE","name":"Niger"},{"alpha2":"NG","name":"Nigeria"},{"alpha2":"NU","name":"Niue"},{"alpha2":"NF","name":"Norfolk Island"},{"alpha2":"MP","name":"Northern Mariana Islands"},{"alpha2":"NO","name":"Norway"},{"alpha2":"OM","name":"Oman"},{"alpha2":"PK","name":"Pakistan"},{"alpha2":"PW","name":"Palau"},{"alpha2":"PS","name":"Palestine, State of"},{"alpha2":"PA","name":"Panama"},{"alpha2":"PG","name":"Papua New Guinea"},{"alpha2":"PY","name":"Paraguay"},{"alpha2":"PE","name":"Peru"},{"alpha2":"PH","name":"Philippines"},{"alpha2":"PN","name":"Pitcairn"},{"alpha2":"PL","name":"Poland"},{"alpha2":"PT","name":"Portugal"},{"alpha2":"PR","name":"Puerto Rico"},{"alpha2":"QA","name":"Qatar"},{"alpha2":"RE","name":"Réunion"},{"alpha2":"RO","name":"Romania"},{"alpha2":"RU","name":"Russian Federation"},{"alpha2":"RW","name":"Rwanda"},{"alpha2":"BL","name":"Saint Barthélemy"},{"alpha2":"SH","name":"Saint Helena, Ascension and Tristan da Cunha"},{"alpha2":"KN","name":"Saint Kitts and Nevis"},{"alpha2":"LC","name":"Saint Lucia"},{"alpha2":"MF","name":"Saint Martin (French part)"},{"alpha2":"PM","name":"Saint Pierre and Miquelon"},{"alpha2":"VC","name":"Saint Vincent and the Grenadines"},{"alpha2":"WS","name":"Samoa"},{"alpha2":"SM","name":"San Marino"},{"alpha2":"ST","name":"Sao Tome and Principe"},{"alpha2":"SA","name":"Saudi Arabia"},{"alpha2":"SN","name":"Senegal"},{"alpha2":"RS","name":"Serbia"},{"alpha2":"SC","name":"Seychelles"},{"alpha2":"SL","name":"Sierra Leone"},{"alpha2":"SG","name":"Singapore"},{"alpha2":"SX","name":"Sint Maarten (Dutch part)"},{"alpha2":"SK","name":"Slovakia"},{"alpha2":"SI","name":"Slovenia"},{"alpha2":"SB","name":"Solomon Islands"},{"alpha2":"SO","name":"Somalia"},{"alpha2":"ZA","name":"South Africa"},{"alpha2":"GS","name":"South Georgia and the South Sandwich Islands"},{"alpha2":"SS","name":"South Sudan"},{"alpha2":"ES","name":"Spain"},{"alpha2":"LK","name":"Sri Lanka"},{"alpha2":"SD","name":"Sudan"},{"alpha2":"SR","name":"Suriname"},{"alpha2":"SJ","name":"Svalbard and Jan Mayen"},{"alpha2":"SZ","name":"Eswatini"},{"alpha2":"SE","name":"Sweden"},{"alpha2":"CH","name":"Switzerland"},{"alpha2":"SY","name":"Syrian Arab Republic"},{"alpha2":"TW","name":"Taiwan, Province of China[a]"},{"alpha2":"TJ","name":"Tajikistan"},{"alpha2":"TZ","name":"Tanzania, United Republic of"},{"alpha2":"TH","name":"Thailand"},{"alpha2":"TL","name":"Timor-Leste"},{"alpha2":"TG","name":"Togo"},{"alpha2":"TK","name":"Tokelau"},{"alpha2":"TO","name":"Tonga"},{"alpha2":"TT","name":"Trinidad and Tobago"},{"alpha2":"TN","name":"Tunisia"},{"alpha2":"TR","name":"Turkey"},{"alpha2":"TM","name":"Turkmenistan"},{"alpha2":"TC","name":"Turks and Caicos Islands"},{"alpha2":"TV","name":"Tuvalu"},{"alpha2":"UG","name":"Uganda"},{"alpha2":"UA","name":"Ukraine"},{"alpha2":"AE","name":"United Arab Emirates"},{"alpha2":"GB","name":"United Kingdom of Great Britain and Northern Ireland"},{"alpha2":"US","name":"United States of America"},{"alpha2":"UM","name":"United States Minor Outlying Islands"},{"alpha2":"UY","name":"Uruguay"},{"alpha2":"UZ","name":"Uzbekistan"},{"alpha2":"VU","name":"Vanuatu"},{"alpha2":"VE","name":"Venezuela (Bolivarian Republic of)"},{"alpha2":"VN","name":"Viet Nam"},{"alpha2":"VG","name":"Virgin Islands (British)"},{"alpha2":"VI","name":"Virgin Islands (U.S.)"},{"alpha2":"WF","name":"Wallis and Futuna"},{"alpha2":"EH","name":"Western Sahara"},{"alpha2":"YE","name":"Yemen"},{"alpha2":"ZM","name":"Zambia"},{"alpha2":"ZW","name":"Zimbabwe"}]
"#;

#[derive(Serialize, Deserialize)]
pub struct AnalyticsTrafficItem {
    pub time: String,
    pub tls: u64,
    pub pv: u64,
}

#[derive(Serialize, Deserialize, Default)]
pub struct TrafficItem {
    pub tls: u64,
    pub pv: u64,
    pub ext_info: ExtInfo,
}

#[derive(Serialize, Deserialize, Default)]
pub struct ExtInfo {
    pub http_country: HashMap<String, u64>,
    pub http_code: HashMap<String, u64>,
    pub http_source: HashMap<String, u64>,
}

#[derive(Serialize, Deserialize)]
struct Iso3116Item {
    alpha2: String,
    name: String,
}

#[derive(Default)]
struct Iso3116ItemMap {
    code_name: HashMap<String, String>,
}

lazy_static! {
    static ref ISO_3116: Iso3116ItemMap = {
        let mut r = Iso3116ItemMap::default();
        if let Ok(items) = serde_json::from_str::<Vec<Iso3116Item>>(ISO_3116_STR) {
            for item in items {
                r.code_name.insert(item.alpha2.clone(), item.name.clone());
            }
        }
        r
    };
}

pub async fn get_analytics_traffic(
    rb: &RBatis,
    cluster_name: &str,
    limit: usize,
) -> anyhow::Result<Vec<AnalyticsTrafficItem>> {
    let items = TbAnalyticsTrafficCluster::select_by_cluster(rb, cluster_name, limit).await?;
    let mut items: Vec<AnalyticsTrafficItem> = items
        .iter()
        .map(|item| {
            let gmt_create = item.gmt_create.as_ref().and_then(|s| {
                chrono::DateTime::from_timestamp(s.unix_timestamp(), 0)
                    .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
            });

            AnalyticsTrafficItem {
                time: gmt_create.map_or("-".to_string(), |s| s),
                tls: item.tls.map_or(0, |i| i),
                pv: item.pv.map_or(0, |i| i),
            }
        })
        .collect();
    items.reverse();
    Ok(items)
}

pub async fn get_analytics_traffic_1day(
    rb: &RBatis,
    cluster_name: &str,
) -> anyhow::Result<Vec<AnalyticsTrafficItem>> {
    let end_time = DateTime::utc();
    let start_time = end_time.clone().sub(Duration::from_secs(86400));

    let mut page_no = 1;

    let mut maps: LinkedHashMap<String, AnalyticsTrafficItem> = LinkedHashMap::new();
    loop {
        let page_request = PageRequest::new(page_no, 100);
        let r = TbAnalyticsTrafficCluster::select_page_by_cluster_and_time(
            rb,
            &page_request,
            cluster_name,
            start_time.clone(),
            end_time.clone(),
        )
        .await?;

        if r.records.is_empty() {
            break;
        }

        for a in r.records {
            let gmt_create = a.gmt_create.as_ref().and_then(|s| {
                chrono::DateTime::from_timestamp(s.unix_timestamp(), 0)
                    .map(|t| t.with_timezone(&chrono::Local).format("%H").to_string())
            });
            if let Some(gmt_create) = gmt_create {
                if !maps.contains(&gmt_create) {
                    maps.insert(
                        gmt_create.clone(),
                        AnalyticsTrafficItem {
                            time: gmt_create,
                            tls: a.tls.map_or(0, |i| i),
                            pv: a.pv.map_or(0, |i| i),
                        },
                    );
                } else if let Some(item) = maps.get_mut(&gmt_create) {
                    item.tls += a.tls.map_or(0, |i| i);
                    item.pv += a.pv.map_or(0, |i| i);
                }
            }
        }
        page_no += 1;
    }

    let mut r = vec![];
    for (_, v) in maps {
        r.push(v);
    }
    r.reverse();

    Ok(r)
}

pub async fn get_analytics_traffic_1month(
    rb: &RBatis,
    cluster_name: &str,
) -> anyhow::Result<Vec<AnalyticsTrafficItem>> {
    let end_time = DateTime::utc();
    let start_time = end_time.clone().sub(Duration::from_secs(86400 * 30));

    let mut page_no = 1;

    let mut maps: LinkedHashMap<String, AnalyticsTrafficItem> = LinkedHashMap::new();
    loop {
        let page_request = PageRequest::new(page_no, 100);
        let r = TbAnalyticsTrafficClusterHour::select_page_by_cluster_and_time(
            rb,
            &page_request,
            cluster_name,
            start_time.clone(),
            end_time.clone(),
        )
        .await?;

        if r.records.is_empty() {
            break;
        }

        for a in r.records {
            let gmt_create = a.gmt_create.as_ref().and_then(|s| {
                chrono::DateTime::from_timestamp(s.unix_timestamp(), 0)
                    .map(|t| t.with_timezone(&chrono::Local).format("%m-%d").to_string())
            });
            if let Some(gmt_create) = gmt_create {
                if !maps.contains(&gmt_create) {
                    maps.insert(
                        gmt_create.clone(),
                        AnalyticsTrafficItem {
                            time: gmt_create,
                            tls: a.tls.map_or(0, |i| i),
                            pv: a.pv.map_or(0, |i| i),
                        },
                    );
                } else if let Some(item) = maps.get_mut(&gmt_create) {
                    item.tls += a.tls.map_or(0, |i| i);
                    item.pv += a.pv.map_or(0, |i| i);
                }
            }
        }
        page_no += 1;
    }

    let mut r = vec![];
    for (_, v) in maps {
        r.push(v);
    }
    r.reverse();

    Ok(r)
}

pub async fn get_analytics_traffic_ext_info_1month(
    rb: &RBatis,
    cluster_name: &str,
) -> anyhow::Result<ExtInfo> {
    let start_time = DateTime::utc().sub(Duration::from_secs(86400 * 30));

    let items = TbAnalyticsTrafficClusterHour::select_by_cluster_gmt_create_greater(
        rb,
        cluster_name,
        start_time,
    )
    .await?;

    let mut ext_info = ExtInfo::default();
    for a in items {
        count_tb_analytics_traffic_cluster2(&mut ext_info, a);
    }

    let mut map = HashMap::new();
    for (k, v) in &ext_info.http_country {
        if let Some(key) = ISO_3116.code_name.get(k) {
            map.insert(key.to_string(), *v);
        }
    }
    ext_info.http_country = map;

    Ok(ext_info)
}

pub(crate) async fn analytics_traffic_minute(
    rb: &RBatis,
    cluster_name: &str,
    task: &Task,
) -> anyhow::Result<Option<TrafficItem>> {
    let access_items = TbAnalyticsTrafficCluster::select_by_cluster_gmt_create(
        rb,
        cluster_name,
        DateTime::from_timestamp(task.end_time.timestamp()),
    )
    .await?;

    let new_end_time = task.end_time + Duration::from_secs(60);

    if access_items.is_none() {
        let mut page_no = 1;

        let mut access_item = TrafficItem::default();
        loop {
            let page_request = PageRequest::new(page_no, 100);
            let r = TbAnalyticsTraffic::select_page_by_cluster_and_time(
                rb,
                &page_request,
                cluster_name,
                DateTime::from_timestamp(task.end_time.timestamp()),
                DateTime::from_timestamp(new_end_time.timestamp()),
            )
            .await?;

            if r.records.is_empty() {
                break;
            }

            for a in r.records {
                count_tb_analytics_traffic(&mut access_item, a);
            }
            page_no += 1;
        }
        Ok(Some(access_item))
    } else {
        Ok(None)
    }
}

pub(crate) async fn analytics_traffic_hour(
    rb: &RBatis,
    cluster_name: &str,
    task: &Task,
) -> anyhow::Result<Option<TrafficItem>> {
    let access_item = TbAnalyticsTrafficClusterHour::select_by_cluster_gmt_create(
        rb,
        cluster_name,
        DateTime::from_timestamp(task.end_time.timestamp()),
    )
    .await?;

    let new_end_time = task.end_time + Duration::from_secs(3600);

    if access_item.is_none() {
        let mut page_no = 1;

        let mut item = TrafficItem::default();
        loop {
            let page_request = PageRequest::new(page_no, 100);
            let r = TbAnalyticsTrafficCluster::select_page_by_cluster_and_time(
                rb,
                &page_request,
                cluster_name,
                DateTime::from_timestamp(task.end_time.timestamp()),
                DateTime::from_timestamp(new_end_time.timestamp()),
            )
            .await?;

            if r.records.is_empty() {
                break;
            }

            for a in r.records {
                count_tb_analytics_traffic_cluster(&mut item, a);
            }
            page_no += 1;
        }
        Ok(Some(item))
    } else {
        Ok(None)
    }
}

fn count_tb_analytics_traffic(item: &mut TrafficItem, a: TbAnalyticsTraffic) {
    item.tls += a.tls.map_or(0, |i| i);
    item.pv += a.pv.map_or(0, |i| i);

    if let Some(country) = &a.http_country {
        let countries: Result<HashMap<String, u64>, serde_json::Error> =
            serde_json::from_str(country);
        if let Ok(contries) = countries {
            for (k, v) in contries {
                *item.ext_info.http_country.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(code) = &a.http_code {
        let codes: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(code);
        if let Ok(codes) = codes {
            for (k, v) in codes {
                *item.ext_info.http_code.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(source) = &a.http_source {
        let sources: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(source);
        if let Ok(sources) = sources {
            for (k, v) in sources {
                *item.ext_info.http_source.entry(k).or_insert(v) += v;
            }
        }
    }
}

fn count_tb_analytics_traffic_cluster(item: &mut TrafficItem, a: TbAnalyticsTrafficCluster) {
    item.tls += a.tls.map_or(0, |i| i);
    item.pv += a.pv.map_or(0, |i| i);

    if let Some(country) = &a.http_country {
        let countries: Result<HashMap<String, u64>, serde_json::Error> =
            serde_json::from_str(country);
        if let Ok(contries) = countries {
            for (k, v) in contries {
                *item.ext_info.http_country.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(code) = &a.http_code {
        let codes: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(code);
        if let Ok(codes) = codes {
            for (k, v) in codes {
                *item.ext_info.http_code.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(source) = &a.http_source {
        let sources: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(source);
        if let Ok(sources) = sources {
            for (k, v) in sources {
                *item.ext_info.http_source.entry(k).or_insert(v) += v;
            }
        }
    }
}

fn count_tb_analytics_traffic_cluster2(ext_info: &mut ExtInfo, a: TbAnalyticsTrafficClusterHour) {
    if let Some(country) = &a.http_country {
        let countries: Result<HashMap<String, u64>, serde_json::Error> =
            serde_json::from_str(country);
        if let Ok(contries) = countries {
            for (k, v) in contries {
                *ext_info.http_country.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(code) = &a.http_code {
        let codes: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(code);
        if let Ok(codes) = codes {
            for (k, v) in codes {
                *ext_info.http_code.entry(k).or_insert(v) += v;
            }
        }
    }

    if let Some(source) = &a.http_source {
        let sources: Result<HashMap<String, u64>, serde_json::Error> = serde_json::from_str(source);
        if let Ok(sources) = sources {
            for (k, v) in sources {
                *ext_info.http_source.entry(k).or_insert(v) += v;
            }
        }
    }
}
