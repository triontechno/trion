#!/usr/bin/env python3
"""Artifact-first TRION image builder; never invokes the full build implicitly."""
import argparse, hashlib, json, os, shutil, subprocess, sys, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
BUILD=ROOT/"build"
def log(s): print("[SimpleBuild] "+s, flush=True)
def fail(s): raise SystemExit("[SimpleBuild] ERROR: "+s)
def digest(p):
 h=hashlib.sha256()
 with p.open("rb") as f:
  for b in iter(lambda:f.read(1024*1024),b""): h.update(b)
 return h.hexdigest()
def configs(arch):
 found={p.stem for p in (BUILD/"config").glob("*.toml")}
 found.update(p.stem for p in (BUILD/"config"/arch).glob("*.toml"))
 return sorted(found)
def config_file(arch,name):
 for p in (BUILD/"config"/arch/(name+".toml"),BUILD/"config"/(name+".toml")):
  if p.is_file(): return p
 fail("No filesystem config for "+arch+"/"+name+"; available: "+", ".join(configs(arch)))
def acquire(installer):
 manifest=ROOT/"simplebuild"/"downloads.json"
 if installer.is_file(): return
 if not manifest.is_file(): fail("redox_installer is missing and no downloads.json is available.")
 data=json.loads(manifest.read_text())
 item=data.get("redox_installer")
 if not item: fail("redox_installer missing; add a trusted HTTPS URL and SHA-256 to simplebuild/downloads.json.")
 url=item.get("url",""); expected=item.get("sha256","").lower()
 if not url.startswith("https://") or len(expected)!=64: fail("Download entry must have HTTPS URL and 64-character SHA-256.")
 installer.parent.mkdir(parents=True,exist_ok=True); tmp=installer.with_suffix(".download")
 log("Downloading missing redox_installer from configured HTTPS source.")
 try:
  req=urllib.request.Request(url,headers={"User-Agent":"TRION-SimpleBuild/1"})
  with urllib.request.urlopen(req,timeout=60) as r,tmp.open("wb") as f: shutil.copyfileobj(r,f)
  if digest(tmp)!=expected: fail("Downloaded installer SHA-256 mismatch; refusing to install it.")
  tmp.replace(installer); installer.chmod(installer.stat().st_mode|0o111)
 finally: tmp.unlink(missing_ok=True)
def changed_recipes():
 git=shutil.which("git")
 if not git or not (ROOT/".git").exists(): return []
 r=subprocess.run([git,"-C",str(ROOT),"status","--porcelain","--untracked-files=no"],capture_output=True,text=True)
 result=set()
 for line in r.stdout.splitlines():
  path=line[3:].strip().replace("\\","/")
  if path.startswith("build/recipes/"):
   parts=path.split("/")
   if len(parts)>3: result.add(parts[3])
 return sorted(result)
def build_one(installer,cfg,out,arch,name,live):
 r=subprocess.run([str(installer),"--filesystem-size","-c",str(cfg)],cwd=ROOT,text=True,capture_output=True,check=True)
 size=r.stdout.strip()
 if not size.isdigit() or int(size)<1: fail("Installer returned invalid filesystem size: "+repr(size))
 out.parent.mkdir(parents=True,exist_ok=True); tmp=out.with_name(out.name+".partial"); tmp.unlink(missing_ok=True)
 with tmp.open("wb") as f: f.truncate(int(size)*1024*1024)
 cmd=[str(installer),"--cookbook=.", "--config-name="+name,"-c",str(cfg)]
 if live: cmd += ["--write-bootloader="+str(BUILD/"build"/arch/name/"bootloader-live.efi"),"--live"]
 cmd.append(str(tmp))
 try:
  subprocess.run(cmd,cwd=ROOT,check=True)
  if not tmp.is_file() or not tmp.stat().st_size: fail("Installer produced empty output: "+str(tmp))
  tmp.replace(out)
 finally: tmp.unlink(missing_ok=True)
def main():
 p=argparse.ArgumentParser()
 p.add_argument("--arch",default=os.getenv("ARCH","x86_64")); p.add_argument("--config",default=os.getenv("CONFIG_NAME","desktop"))
 p.add_argument("--list-configs",action="store_true"); p.add_argument("--no-rebuild-changed",action="store_true")
 p.add_argument("--qemu",action="store_true"); p.add_argument("--memory",default="4096")
 a=p.parse_args()
 if a.list_configs:
  print("\n".join(configs(a.arch))); return
 cfg=config_file(a.arch,a.config); installer=BUILD/"fstools"/"bin"/"redox_installer"
 acquire(installer)
 changed=changed_recipes()
 if changed and a.no_rebuild_changed: fail("Changed recipe sources: "+", ".join(changed))
 if changed:
  make=shutil.which("make")
  if not make: fail("Changed recipes found but GNU make is unavailable: "+", ".join(changed))
  for recipe in changed:
   log("Rebuilding changed recipe only: "+recipe)
   subprocess.run([make,"-C",str(BUILD),"ARCH="+a.arch,"CONFIG_NAME="+a.config,"PODMAN_BUILD=0","r."+recipe],check=True)
 if not (ROOT/"repo").exists(): fail("Existing repo/ package repository is missing; refusing to trigger Cookbook/full compilation.")
 out=BUILD/"build"/a.arch/a.config
 log("Using existing Redox installer and package repository; Podman is bypassed.")
 log("Backend C selected: preserve native RedoxFS/bootloader semantics until an independent assembler is benchmarked faster.")
 build_one(installer,cfg,out/"harddrive.img",a.arch,a.config,False)
 build_one(installer,cfg,out/"redox-live.iso",a.arch,a.config,True)
 artifacts={}
 for name in ("harddrive.img","redox-live.iso"):
  f=out/name
  if not f.is_file() or not f.stat().st_size: fail("Output missing or empty: "+str(f))
  artifacts[name]={"size":f.stat().st_size,"sha256":digest(f)}
 (out/"SHA256SUMS").write_text("".join(v["sha256"]+"  "+k+"\n" for k,v in artifacts.items()))
 cache=BUILD/"simplebuild"/"cache"; cache.mkdir(parents=True,exist_ok=True)
 state={"arch":a.arch,"config":a.config,"config_sha256":digest(cfg),"artifacts":artifacts,"backend":"redox_installer"}
 (cache/"manifest.json").write_text(json.dumps(state,indent=2)+"\n")
 log("Image generation and SHA-256 validation succeeded.")
 for k,v in artifacts.items(): log(k+": "+str(v["size"])+" bytes; sha256="+v["sha256"])
 if a.qemu:
  qemu=shutil.which("qemu-system-x86_64" if a.arch=="x86_64" else "qemu-system-"+a.arch)
  if not qemu: fail("Matching qemu-system executable not found.")
  log("Starting interactive QEMU boot attempt; close the VM to return.")
  subprocess.run([qemu,"-m",a.memory,"-cdrom",str(out/"redox-live.iso"),"-boot","d","-serial","stdio"],check=False)
if __name__=="__main__":
 try: main()
 except subprocess.CalledProcessError as e: fail("Command failed with exit code "+str(e.returncode)+": "+str(e.cmd))
