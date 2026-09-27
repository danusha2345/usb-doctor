#!/usr/bin/env python3
"""Сравнить приватный raw-снимок с USBTreeView XML; выводить только агрегаты."""
import argparse
import collections
import json
import re
import xml.etree.ElementTree as ET

parser = argparse.ArgumentParser()
parser.add_argument('snapshot')
parser.add_argument('reference')
args = parser.parse_args()
with open(args.snapshot, encoding='utf-8-sig') as file:
    snapshot = json.load(file)
reference = collections.defaultdict(lambda: {'speeds': [], 'config_lengths': []})
for node in ET.parse(args.reference).iter('node'):
    text = node.findtext('text', '')
    vendor = re.search(r'idVendor\s*:\s*0x([0-9a-f]+)', text, re.I)
    product = re.search(r'idProduct\s*:\s*0x([0-9a-f]+)', text, re.I)
    if not (vendor and product):
        continue
    key = (int(vendor[1], 16), int(product[1], 16))
    speed = re.search(r'Device Connection Speed\s*:\s*([^\r\n]+)', text)
    reference[key]['speeds'].append(speed[1].strip() if speed else 'unknown')
    lengths = re.findall(r'Configuration Descriptor[^\r\n]*[\s\S]*?wTotalLength\s*:\s*0x([0-9a-f]+)', text, re.I)
    reference[key]['config_lengths'].extend(int(value, 16) for value in lengths)
actual = collections.defaultdict(lambda: {'speeds': [], 'config_lengths': []})
for device in snapshot['devices']:
    key = (device['vendor_id'], device['product_id'])
    flags = device['speed']['v2_flags']
    ex = device['speed']['ex_speed']
    if flags is None:
        speed = 'unknown'
    elif flags & 4:
        rate = next((field['value'].split()[0] for field in device['fields'] if field['id'] == 'usb.ssp.RX.rate'), '?')
        speed = 'SuperSpeedPlus ' + rate + ' GBit/s'
    elif flags & 1:
        speed = 'SuperSpeed'
    else:
        speed = {0: 'Low-Speed', 1: 'Full-Speed', 2: 'High-Speed'}.get(ex, 'unknown')
    actual[key]['speeds'].append(speed)
    actual[key]['config_lengths'].extend(len(item['raw']) for item in device['descriptors'] if item['kind'] in ('Configuration', 'Other speed configuration'))
for values in (reference, actual):
    for item in values.values():
        for key in item:
            item[key].sort()
result = {
    'devices': len(snapshot['devices']),
    'topology': len(snapshot['topology']),
    'fields': sum(len(d['fields']) for d in snapshot['devices'] + snapshot['topology']),
    'descriptor_blocks': sum(len(d['descriptors']) for d in snapshot['devices']),
    'reference_instances': sum(len(v['speeds']) for v in reference.values()),
    'inventory_complete': snapshot['inventory_complete'],
    'global_errors': len(snapshot['issues']),
    'match_vid_pid_multiplicity_speed_and_configuration_lengths': bool(reference) and reference == actual,
    'hid_records': sum(sum('hid.' in f['id'] for f in d['fields']) for d in snapshot['devices']),
}
print(json.dumps(result, ensure_ascii=False, indent=2))
raise SystemExit(0 if result['match_vid_pid_multiplicity_speed_and_configuration_lengths'] and snapshot['inventory_complete'] else 1)
