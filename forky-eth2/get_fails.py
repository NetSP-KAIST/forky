import re
import os
from bs4 import BeautifulSoup

# Read the file
with open('logs_prysm', 'r') as file_prysm:
    prysm_content = file_prysm.read()
with open('logs_nimbus', 'r') as file_nimbus:
    nimbus_content = file_nimbus.read()
with open('logs_lighthouse', 'r') as file_lighthouse:
    lighthouse_content = file_lighthouse.read()

# teku_lines = []
# with open('logs_teku.txt', 'r') as file_teku:
#     teku_lines = file_teku.readlines()

# Read the HTML file from teku
with open('teku-23.6.2/eth-reference-tests/build/reports/tests/referenceTest/index.html', 'r') as file_teku:
    teku_content = file_teku.read()

# Extract the failed test cases hashes
prysm_fail = re.findall(r"FAIL: TestMainnet_Capella_Forkchoice/(\w+)\s", prysm_content)
prysm_fails = list(set(prysm_fail))
print("Prysm fails\n", prysm_fails)

nimbus_fail = re.findall(r"\[FAILED\] ForkChoice - mainnet/capella/fork_choice/forky/pyspec_tests/(\w+)", nimbus_content)
nimbus_fails = list(set(nimbus_fail))
print("Nimbus fails\n", nimbus_fails)

lighthouse_fail = re.findall(r"case \d+ \((.*?)\) from", lighthouse_content)
lighthouse_fails = list(set(lighthouse_fail))
print("Lighthouse fails\n", lighthouse_fails)

# teku_fail = [fail.rstrip('\n') for fail in teku_lines]
# teku_fails = list(set(teku_fail))
# # print(teku_fails)

# Parse the HTML content
teku_bs = BeautifulSoup(teku_content, 'html.parser')
# Find all <td> elements with class "failures"
failure_td_elements = teku_bs.find_all('td', class_='failures')
teku_fail = []
for td_element in failure_td_elements:
    a_element = td_element.find('a')
    if a_element:
        href = a_element['href']
        # print(href)
        test_case_hash = href.replace("classes/tech.pegasys.teku.reference.capella.forky.Testcase","").replace(".html","")
        teku_fail.append(test_case_hash)
if "packages/tech.pegasys.teku.reference.capella.forky" in teku_fail:
    teku_fail.remove("packages/tech.pegasys.teku.reference.capella.forky")
teku_fails = list(set(teku_fail))
print("Teku fails\n", teku_fails)

all_fails = list(set(prysm_fails) | set(nimbus_fails) | set(teku_fails) | set(lighthouse_fails))

common_fails = list(set(prysm_fails) & set(nimbus_fails) & set(teku_fails) & set(lighthouse_fails))

prysm_nimbus_fails = list(set(prysm_fails) & set(nimbus_fails))
prysm_teku_fails = list(set(prysm_fails) & set(teku_fails))
prysm_lighthouse_fails = list(set(prysm_fails) & set(lighthouse_fails))
nimbus_lihthouse_fails = list(set(nimbus_fails) & set(lighthouse_fails))
nimbus_teku_fails = list(set(nimbus_fails) & set(teku_fails))
teku_lighthouse_fails = list(set(teku_fails) & set(lighthouse_fails))

prysm_fails_only = list(set(prysm_fails) - set(nimbus_fails) - set(teku_fails) - set(lighthouse_fails))
nimbus_fails_only = list(set(nimbus_fails) - set(prysm_fails) - set(teku_fails) - set(lighthouse_fails))
teku_fails_only = list(set(teku_fails) - set(prysm_fails) - set(nimbus_fails) - set(lighthouse_fails))
lighthouse_fails_only = list(set(lighthouse_fails) - set(prysm_fails) - set(nimbus_fails) - set(teku_fails))


print("All fails:\n", all_fails, "\n\n")

print("Common fails:\n", common_fails)

print("Prysm and Nimbus fails:\n", prysm_nimbus_fails)
print("Prysm and Teku fails:\n", prysm_teku_fails)
print("Nimbus and Teku fails:\n", nimbus_teku_fails)
print("Prysm and Lighthouse fails:\n", prysm_lighthouse_fails)
print("Nimbus and Lighthouse fails:\n", nimbus_lihthouse_fails)
print("Teku and Lighthouse fails:\n", teku_lighthouse_fails, "\n\n")

print("Prysm fails only:\n", prysm_fails_only)
print("Nimbus fails only:\n", nimbus_fails_only)
print("Teku fails only:\n", teku_fails_only)


os.system("mkdir -p failed_testcases")
for i in all_fails:
    os.system("cp -r testcases/" + i + " ./failed_testcases") 