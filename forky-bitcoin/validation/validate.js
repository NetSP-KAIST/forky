var path = require('path');
var fs = require('fs');
var crypto = require('crypto');
const { format } = require('path');

const cores = ["bitcoin", "knots", "bcoin", "btcd"];
const versions = {
    "bitcoin": ["27.1.0", "26.0.0", "25.0.0", "24.1.0", "23.0.0", "22.0.0",
    "0.21.2", "0.21.1", "0.21.0", "0.20.1", "0.20.0", 
    // "0.19.1", "0.19.0.1", "0.19.0", "0.18.1", "0.18.0", "0.17.1", "0.17.0.1", "0.17.0", 
    "0.19.1", "0.19.0.1", "0.19.0", "0.18.1", "0.18.0", "0.17.1", "0.17.0", 
    "0.16.3", "0.16.2", "0.16.1", "0.16.0", "0.15.2", "0.15.1", "0.15.0"],
    "knots": ["23.0.0", "22.0.0"],
    "bcoin": ["2.2.0", "2.1.0", "2.0.0"],
    "btcd": ["0.23.1", "0.23.0", "0.22.1"]
}

function deep(result) {
    return result;    
}

function reason(result) {

    reasons = {}

    for (const key of Object.keys(result)) {
        if (key === "HashTip" || key === "UTXO") continue;
        reasons[key] = result[key]["reason"]
    }

    return reasons;
}

function shallow(result) {
    
    for (const key of Object.keys(result)) {
        if (key === "HashTip" || key === "UTXO") continue;
        delete result[key]["reason"];
    }

    return result;
}

function accept(result) {

    accepts = {}

    for (const key of Object.keys(result)) {
        if (key === "HashTip" || key === "UTXO") continue;
        accepts[key] = result[key]["accept"]
    }

    return accepts;
}

function hashtip(result) {
    return result["HashTip"];
}

function utxo(result) {
    return result["UTXO"];
}

function utxo_with_tags(result) {
    return result["UTXO"];
}

const formats = [deep, utxo_with_tags, reason, shallow, accept, hashtip, utxo];

validate("./results", "./test_cases");

function validate(res_path, test_path) {

    const test_core_ver_hash = {};

    const test_files = fs.readdirSync(test_path);

    for (const test_file of test_files) {

        test_core_ver_hash[test_file] = {};

        for (const core of cores) {

            test_core_ver_hash[test_file][core] = {};
            
            for (const version of versions[core]) {
                test_core_ver_hash[test_file][core][version] = {};
            }
        }
    }

    for (const core of cores) {
        for (const version of versions[core]) {

            const dir_path = path.join(res_path, path.join(core, version));
            const res_files = fs.readdirSync(dir_path); 

            for (const res_file of res_files) {

                let result;

                try {
                    result = JSON.parse(fs.readFileSync(path.join(dir_path, res_file)));
                } catch {
                    continue;
                }

                for (const format of formats) {

                    if (format.name === 'utxo_with_tags' && core !== 'bitcoin' && core !== 'knots') continue;

                    var hash = crypto.createHash('sha1');
                    const res_hash = hash.update(JSON.stringify(format(result), null, 2)).digest().toString('hex');

                    if (core === "bitcoin" || core === "knots") {
                        test_core_ver_hash[res_file.trim().split('_')[0]][core][version][format.name] = res_hash;
                        if (format.name === "utxo_with_tags") parse(result);
                    } else {
                        test_core_ver_hash[res_file][core][version][format.name] = res_hash;
                    }
                }
            }
        }
    }

    for (const test_file of test_files) {
        for (const core of cores) {

            for (const [version, hash] of Object.entries(test_core_ver_hash[test_file][core])) {
                if (!hash[formats[0].name]) log_crash(test_file, core, version);
            }
            
            for (const [index, [version, hash]] of Object.entries(test_core_ver_hash[test_file][core]).slice(1).entries()) {
                
                let hash1 = test_core_ver_hash[test_file][core][versions[core][index]][formats[0].name];
                let hash2 = hash[formats[0].name];

                if (!hash1 || !hash2) continue;
    
                if (hash1 !== hash2) {
                    
                    const mismatched = []

                    for (const format of formats.slice(2)) {
                        
                        if (format.name === "shallow") continue;

                        hash1 = test_core_ver_hash[test_file][core][versions[core][index]][format.name]
                        hash2 = hash[format.name];
    
                        if (hash1 != hash2) mismatched.push(format.name)
                    }

                    if (core === 'bitcoin' || core === 'knots') {
                        hash1 = test_core_ver_hash[test_file][core][versions[core][index]]['utxo_with_tags']
                        hash2 = hash['utxo_with_tags'];
    
                        if (hash1 != hash2) mismatched.push('utxo_with_tags')
                    } 
    
                    log_mismatch(test_file, core + ":" + versions[core][index], core + ":" + version, mismatched); 
                }
            }
        }
    }

    for (const test_file of test_files) {
        for (const core of cores.slice(1)) {

            let hash1 = test_core_ver_hash[test_file][cores[0]][versions[cores[0]][0]][formats[3].name];
            let hash2 = test_core_ver_hash[test_file][core][versions[core][0]][formats[3].name];

            if (!hash1 || !hash2) continue;

            if (hash1 !== hash2) {
                
                const mismatched = []

                for (const format of formats.slice(4)) {

                    hash1 = test_core_ver_hash[test_file][cores[0]][versions[cores[0]][0]][format.name]
                    hash2 = test_core_ver_hash[test_file][core][versions[core][0]][format.name];

                    if (hash1 != hash2) mismatched.push(format.name)
                }

                log_mismatch(test_file, cores[0], core, mismatched); 
            }
        }
    }
}

function parse(result) {

    const utxos = [];
    
    for (const key of Object.keys(result["UTXO"])) utxos.push(key);

    result["UTXO"] = utxos;
}

function log_mismatch(testcase, core1, core2, mismatched) {
    fs.appendFileSync('/log/log.txt', testcase + " | MISMATCH: " + core1 + " & " + core2 + " | mismatch in: " + mismatched + "\n");
    console.log(testcase + " | MISMATCH: " + core1 + " & " + core2 + " | mismatch in: " + mismatched);
}

function log_crash(testcase, core, version) {
    fs.appendFileSync('/log/log.txt', testcase + " | INCORRECT OUTPUT: " + core + ":" + version + "\n");
    console.log(testcase + " | INCORRECT OUTPUT: " + core + ":" + version);
}