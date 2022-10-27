use std::fs::File;
use std::fs;
use std::io::{BufWriter, BufReader, Result, Write};
use std::process::Command;
use std::str;
use std::iter::FromIterator;

use tempfile::Builder;

use geo_types::{Polygon, GeometryCollection};
use geo::Geometry;

use wkt::{TryFromWkt, ToWkt};
use geojson::FeatureCollection;


use flatgeobuf::{FallibleStreamingIterator, FeatureProperties, FgbReader};

use geozero::ToGeo;
// use seek_bufread::BufReader;

pub fn prep_plot_geojson(poly: Polygon) -> FeatureCollection {
    let polys = vec![poly];
    let gc = GeometryCollection::from_iter(polys);
    FeatureCollection::from(&gc)
}


pub fn convert_wkt_to_geotype(txt: String) -> Polygon {
    let t_geo: Polygon<f64> = Polygon::try_from_wkt_str(&txt).unwrap();
    t_geo
}


pub fn metafile(resolution: i32, work_dir: &str) -> String {
    let mut tpl = String::new();

    let t1 = "# specify the operation
dggrid_operation GENERATE_GRID
dggs_type ISEA7H
dggs_res_spec ";

    tpl.push_str(t1);

    let res = resolution.to_string();

    tpl.push_str(&res);
    tpl.push('\n');

    let t2 = "clip_subset_type GDAL
clip_region_files ";

    tpl.push_str(t2);
    tpl.push_str(work_dir);
    tpl.push_str("/plot.geojson\n");

    let t4 = "# increase granularity of the clipping algorithm for high res
clipper_scale_factor 10000000

# specify the output using GDAL-supported file formats
cell_output_type GDAL
cell_output_gdal_format FlatGeobuf
cell_output_file_name ";

    tpl.push_str(t4);
    tpl.push_str(work_dir);
    tpl.push_str("/cells.fgb\n");

    let t5 = "# point_output_type GDAL
# point_output_gdal_format FlatGeobuf
# point_output_file_name points.fgb
densification 0
precision 6";
    tpl.push_str(t5);

    tpl
}


fn read_check_textfile(fname: &str) {
    let data = fs::read_to_string(&fname).expect("Unable to read file");
    println!("{}", data);
}


fn main() -> Result<()> {
    let tmp_dir = Builder::new()
        .prefix("dg_exec_")
        .tempdir()
        .expect("Unable to create TempDir");

    let tmp_path = tmp_dir.path().to_owned();

    let meta_name = "metafile";
    let meta_tmp = tmp_path.join(&meta_name);
    let meta_path = &meta_tmp.as_path();

    let json_name = "plot.geojson";
    let json_tmp = tmp_path.join(&json_name);
    let json_path = json_tmp.as_path();

    let cells_name = "cells.fgb";
    let cells_tmp = tmp_path.join(&cells_name);
    let cells_path = cells_tmp.as_path();

    // let tmpdir = "/tmp";
    let workdir = tmp_path.into_os_string().into_string().unwrap();
    let resolution = 14i32;

    let meta_out = metafile(resolution, &workdir);

    let f = File::create(&meta_path).expect("Unable to create meta file");
    let mut f = BufWriter::new(f);
    f.write_all(meta_out.as_bytes()).expect("Unable to write meta data");
    f.flush().unwrap();

    let plot_wkt_geom: Polygon = convert_wkt_to_geotype(String::from("POLYGON((25.87759862085369 58.53555491455084,25.87285123193909 58.5340925807705,25.87184652637221 58.534965460089424,25.87092254284933 58.535765119198125,25.87256208729135 58.536213900430916,25.872627063444387 58.53623091521619,25.873038753011613 58.53636))"));

    let js_geo = prep_plot_geojson(plot_wkt_geom);
    let js_geo_out = js_geo.to_string();

    let f2 = File::create(&json_path).expect("Unable to create js file");
    let mut f2 = BufWriter::new(f2);
    f2.write_all(js_geo_out.as_bytes()).expect("Unable to write js data");
    f2.flush().unwrap();

    let meta_path_clone = meta_tmp.clone();
    let meta_path_str = meta_path_clone.to_str().unwrap();

    read_check_textfile(meta_path_str);
    
    let json_path_clone = json_tmp.clone();
    let json_path_str = json_path_clone.to_str().unwrap();

    read_check_textfile(json_path_str);

    let output = Command::new("dggrid75")
        .arg(&meta_path_str)
        .output()
        .expect("failed to execute process");

    let _hello = match str::from_utf8(&output.stdout) {
        Ok(val) => val,
        Err(_) => panic!("Got none or not readable data from dggrid exec"),
    };

    let cells_path_str = cells_path.to_str().unwrap();
    let mut file = BufReader::new(File::open(cells_path_str).expect("unable to use fgb file"));
    let fgb = FgbReader::open(&mut file).expect("unable to open fgb file");
    let mut fgb_sel = fgb.select_all().expect("unable to open select fgb data");

    let mut table: Vec<(i64, String)> = Vec::new();
     
    while let Some(feature) = fgb_sel.next().unwrap() {
        let name: String = feature.property("name").unwrap();
        if let Ok(Geometry::Polygon(poly)) = feature.to_geo() {
            let wkt = poly.to_wkt();
            let wkt_str = wkt.to_string();
            let name_int: i64 = name.parse::<i64>().unwrap();
            table.push( (name_int, wkt_str) );
            // println!("{} : {}", name_int, wkt_str);
        }
    };

    Ok(())
}
