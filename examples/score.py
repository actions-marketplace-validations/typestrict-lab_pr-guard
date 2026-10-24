"""Scoreboard: every scenario through pr-guard against expected.yaml (complexity counts within one band). Needs PyYAML."""
import subprocess,glob,os,re,json,yaml,collections
import sys
B=os.environ.get('PR_GUARD','target/release/pr-guard'); tot=collections.Counter(); hit=collections.Counter(); rows=[]
for d in sorted(glob.glob('examples/*/scenarios/*/')):
    e=yaml.safe_load(open(d+'expected.yaml'))
    r=subprocess.run([B,'check','--diff',d+'change.diff','--description-file',d+'description.md','--policy','/tmp/nao-existe.yml'],capture_output=True,text=True)
    p={}
    for l in r.stderr.splitlines():
        m=re.match(r'pr-guard: (\S+)\s+([\d.]+)\s+(.*)',l)
        if m: p[m[1]]=(float(m[2]),m[3])
    yes=lambda t: p[t][1].startswith('Yes')
    got={'description_matches':'yes' if yes('pr-descricao-noul') else 'no','vulnerability':'yes' if yes('prg-vul') else 'no','breaking':'yes' if yes('prg-quebra') else 'no',
         'tests':{'Yes':'covers','No':'missing','Not':'not_applicable'}[p['prg-testes'][1].split()[0].rstrip(',:')],'type':p['prg-tipo'][1],
         'complexity':['Trivial','Small','Medium','Large','Very'].index(p['prg-complexidade'][1].split(':')[0].split()[0])}
    line=[]
    for k in got:
        exp=e.get(k); exp='yes' if exp is True else 'no' if exp is False else exp
        ok = abs(int(got[k])-int(exp))<=1 if k=='complexity' else str(got[k])==str(exp)
        tot[k]+=1; hit[k]+=ok
        if not ok: line.append(f'{k}: {got[k]} (expected {exp})')
    rows.append((d.split('examples/')[1].rstrip('/'),line))
for n,l in rows: print(n, '| OK' if not l else '| '+'; '.join(l))
print({k:f'{hit[k]}/{tot[k]}' for k in tot})
