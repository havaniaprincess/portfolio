use std::process::Command;



/* pub fn get_text(path: &String, tessdata: Option<&str>, lang: &String) -> String {
    let mut lt = LepTess::new(tessdata, lang).unwrap();
    let _ = lt.set_image(path);
    let text = lt.get_utf8_text().unwrap().to_lowercase();
    text
} */

pub fn get_box_learn(path: &String) {
    let tess_path =  match std::env::var("TESSDIR") {
        Ok(val) => val,
        Err(e) => panic!("Couldn't read TESSDIR: {}", e),
    };
    //dbg!(&tess_path);
    let status = Command::new(tess_path + "/tesseract.exe")
        .arg(path)     // входной файл
        .arg(path.to_string() + ".b")       // имя выходного файла (без расширения)
        .arg("-l")
        .arg("rus")  
        .arg("batch.nochop")  
        .arg("makebox")          // язык
        .status()
        .expect("Ошибка запуска Tesseract");

    dbg!(status.success());
}