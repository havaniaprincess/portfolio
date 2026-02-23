
const fs = require('fs');
const csv = require('csv-parser');
const path = require('path');

let paths = [
    "../database_data/fishes",
    "../database_data/maps/losinoe",
    "../database_data/tests"
];
paths.forEach(path_item => {
    
    fs.readdir(path_item, (err, files) => {
    if (err) {
        console.error('Ошибка при чтении папки:', err);
        return;
    }

    // Отфильтровать только файлы (не папки)
    files.forEach(file => {
        const fullPath = path.join(path_item, file);
        if (fs.statSync(fullPath).isFile() && file.endsWith(".csv")) {
            let result = [];
            fs.createReadStream(fullPath)
                .pipe(csv({ separator: ';' })) // можно указать свой разделитель
                .on('data', (row) => {
                    row.exp_real = row.exp;
                    let rig_mul = row.device == "match_losinoe_default" ? 1.82 : 1.25;
                    let l_mul = row.device == "match_losinoe_default" ? 0.25 : 0.0;
                    let happy_mul = row.exp_happy > 0 ? 2.0 : 0.0;
                    let prem_mul = row.exp_prem > 0 ? 1.0 : 0.0;
                    let exp_clear = row.exp / rig_mul;
                    row.exp = Math.round(exp_clear);
                    row.exp_l = Math.round(exp_clear * l_mul);
                    row.exp_happy = Math.round(exp_clear * happy_mul);
                    row.exp_drink = Math.round(row.exp_drink / rig_mul);
                    row.exp_prem = Math.round(exp_clear * prem_mul);
                    row.exp_sum = Math.round(exp_clear + exp_clear * l_mul + row.exp_drink + exp_clear * happy_mul + exp_clear * prem_mul);
                    result.push(row);
                    //console.log(row);
                })
                .on('end', () => {
                    const createCsvWriter = require('csv-writer').createObjectCsvWriter;
                        console.log(result);

                    const csvWriter = createCsvWriter({
                        path: fullPath,
                        header: [
                            { id: 'name', title: 'name' },
                            { id: 'test', title: 'test' },
                            { id: 'map', title: 'map' },
                            { id: 'point', title: 'point' },
                            { id: 'timestamp', title: 'timestamp' },
                            { id: 'mass', title: 'mass' },
                            { id: 'long', title: 'long' },
                            { id: 'exp', title: 'exp' },
                            { id: 'exp_l', title: 'exp_l' },
                            { id: 'exp_happy', title: 'exp_happy' },
                            { id: 'exp_prem', title: 'exp_prem' },
                            { id: 'exp_sum', title: 'exp_sum' },
                            { id: 'exp_drink', title: 'exp_drink' },
                            { id: 'device', title: 'device' },
                            { id: 'exp_real', title: 'exp_real' },
                        ],
                        fieldDelimiter: ';'
                    });

                    csvWriter.writeRecords(result)//.pipe(csv({ separator: ';' }))
                    .then(() => {
                        console.log('CSV-файл записан.');
                    });

                    console.log('CSV-файл прочитан.');
                });
        }
    });
    });
})



