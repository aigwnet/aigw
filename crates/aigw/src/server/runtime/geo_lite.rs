use std::net::IpAddr;

use maxminddb::{Reader, geoip2::Country};

pub(crate) struct GeoLite {
    reader: Reader<Vec<u8>>,
}

impl GeoLite {
    pub fn new(path: &str) -> anyhow::Result<Self> {
        let reader =
            Reader::open_readfile(path).map_err(|_| anyhow::anyhow!("Read geolite file error"))?;
        Ok(Self { reader })
    }

    pub fn country(&self, address: IpAddr) -> anyhow::Result<String> {
        let country = self.reader.lookup::<Country>(address)?;
        if let Some(country) = country
            && let Some(country) = country.country
        {
            return country
                .iso_code
                .map_or(Err(anyhow::anyhow!("ISO Code not exist.")), |s| {
                    Ok(s.to_string())
                });
        }
        Err(anyhow::anyhow!("Country not found."))
    }
}
