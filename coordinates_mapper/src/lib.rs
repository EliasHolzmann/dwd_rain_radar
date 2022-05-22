// Calculations are based on https://docs.wradlib.org/en/stable/notebooks/radolan/radolan_grid.html as well as trial and error. Damn, geometry is hard!

const RADIUS_OF_EARTH: f64 = 6370.040;
const LONGITUDE_OF_PROJECTION_ORIGIN: f64 = 10.;
const LATITUDE_OF_TRUE_SCALE: f64 = 60.;
const OFFSET_X: f64 = 542.962_166_921_856_4;
const OFFSET_Y: f64 = -3_609.144_724_265_575;

/// Coordinates in Latitude and Longitude
#[derive(Debug, Copy, Clone)]
pub struct GeographicCoordinates {
    /// N/S
    pub latitude: f64,
    /// W/E
    pub longitude: f64,
}

/// Coordinates on the weather map of DWD
#[derive(Debug, Copy, Clone)]
pub struct StereographicCoordinates {
    pub x: f64,
    pub y: f64,
}

impl From<GeographicCoordinates> for StereographicCoordinates {
    fn from(other: GeographicCoordinates) -> StereographicCoordinates {
        StereographicCoordinates {
            x: -RADIUS_OF_EARTH * (1f64 + (LATITUDE_OF_TRUE_SCALE).to_radians().sin())
                / (1f64 + other.latitude.to_radians().sin())
                * other.latitude.to_radians().cos()
                * (LONGITUDE_OF_PROJECTION_ORIGIN - other.longitude)
                    .to_radians()
                    .sin()
                + OFFSET_X,
            y: RADIUS_OF_EARTH * (1f64 + (LATITUDE_OF_TRUE_SCALE).to_radians().sin())
                / (1f64 + other.latitude.to_radians().sin())
                * other.latitude.to_radians().cos()
                * (LONGITUDE_OF_PROJECTION_ORIGIN - other.longitude)
                    .to_radians()
                    .cos()
                + OFFSET_Y,
        }
    }
}

impl From<StereographicCoordinates> for GeographicCoordinates {
    fn from(other: StereographicCoordinates) -> GeographicCoordinates {
        GeographicCoordinates {
            longitude: ((other.x - OFFSET_X) / (other.y - OFFSET_Y))
                .atan()
                .to_degrees()
                + LONGITUDE_OF_PROJECTION_ORIGIN,

            latitude: ((RADIUS_OF_EARTH.powi(2)
                * (1. + LONGITUDE_OF_PROJECTION_ORIGIN.to_radians().sin()).powi(2)
                + ((other.x - OFFSET_X).powi(2) + (other.y - OFFSET_Y).powi(2)))
                / (RADIUS_OF_EARTH.powi(2)
                    * (1. + LONGITUDE_OF_PROJECTION_ORIGIN.to_radians().sin()).powi(2)
                    - ((other.x - OFFSET_X).powi(2) + (other.y - OFFSET_Y).powi(2))))
            .asin()
            .to_degrees(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! assert_float_eq {
        // Explicit epsilon, fail.
        ($a:expr, $b:expr) => {{
            let (a, b) = ($a, $b);
            if (a - b).abs() > 1.0e-4 {
                panic!(
                    "assert_float_eq failed comparing {} to {}: {} != {}",
                    stringify!($a),
                    stringify!($b),
                    a,
                    b
                );
            }
        }};
    }

    #[test]
    fn test_conversions() {
        let coordinates_from_both_directions: Vec<(
            GeographicCoordinates,
            StereographicCoordinates,
        )> = vec![
            (
                GeographicCoordinates {
                    latitude: 51.,
                    longitude: 9.,
                },
                StereographicCoordinates { x: 469.5, y: 599.5 },
            ),
            (
                GeographicCoordinates {
                    latitude: 55.862143,
                    longitude: 1.4445428,
                },
                StereographicCoordinates { x: 0., y: 0. },
            ),
        ];

        for (geo_coords, stereo_coords) in coordinates_from_both_directions {
            let converted_stereo_coords: StereographicCoordinates = geo_coords.into();
            let converted_geo_coords: GeographicCoordinates = stereo_coords.into();

            assert_float_eq!(converted_stereo_coords.x, stereo_coords.x);
            assert_float_eq!(converted_stereo_coords.y, stereo_coords.y);
            assert_float_eq!(converted_geo_coords.longitude, geo_coords.longitude);
            assert_float_eq!(converted_geo_coords.latitude, geo_coords.latitude);
        }
    }
}
