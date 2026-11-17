#!/usr/bin/env python3
"""Artifact-first TRION image builder; never invokes the full build implicitly."""
import argparse, hashlib, json, os, shutil, subprocess, sys, urllib.request
from pathlib import Path
ROOT=Path(__file__).resolve().parent.parent
BUILD=ROOT/"build"
CACHE=BUILD/"simplebuild"/"cache"/"manifest.json"
def log(s): print("[SimpleBuild] "+s, flush=True)
def fail(s): raise SystemExit("[SimpleBuild] ERROR: "+s)
def digest(p):
 h=hashlib.sha256()
 with p.open("rb") as f:
  for b in iter(lambda:f.read(1024*1024),b""): h.update(b)
 return h.hexdigest()
def git_output(*args):
 git=shutil.which("git")
 if not git or not (ROOT/".git").exists(): return ""
 r=subprocess.run([git,"-C",str(ROOT),*args],capture_output=True,text=True)
 return r.stdout.strip() if r.returncode==0 else ""
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
 item=json.loads(manifest.read_text()).get("redox_installer")
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
def changed_recipes(previous_head):
 changed=set()
 if previous_head:
  names=git_output("diff","--name-only",previous_head+"..HEAD","--","build/recipes").splitlines()
  for path in names:
   parts=path.split("/")
   if len(parts)>3: changed.add(parts[3])
 for line in git_output("status","--porcelain","--untracked-files=no").splitlines():
  path=line[3:].strip().replace("\\","/")
  if path.startswith("build/recipes/"):
   parts=path.split("/")
   if len(parts)>3: changed.add(parts[3])
 return sorted(changed)
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
def cached_outputs_valid(state,arch,name,cfg,installer,out,head):
 if not state or state.get("arch")!=arch or state.get("config")!=name: return False
 if state.get("config_sha256")!=digest(cfg) or state.get("installer_sha256")!=digest(installer): return False
 if state.get("source_revision")!=head: return False
 for filename in ("harddrive.img","redox-live.iso"):
  f=out/filename; old=state.get("artifacts",{}).get(filename,{})
  if not f.is_file() or not f.stat().st_size or digest(f)!=old.get("sha256"): return False
 return True
def qemu_boot(arch,image,memory):
 qemu_name="qemu-system-x86_64" if arch=="x86_64" else "qemu-system-"+arch
 qemu=shutil.which(qemu_name)
 if not qemu: fail("Matching QEMU executable not found: "+qemu_name)
 firmware_candidates=[Path("/usr/share/OVMF/OVMF_CODE.fd"),Path("/usr/share/ovmf/OVMF.fd"),
  Path("/usr/share/qemu/edk2-x86_64-code.fd"),Path("/usr/share/OVMF/OVMF_CODE.fd")]
 firmware=next((p for p in firmware_candidates if p.is_file()),None)
 if arch=="x86_64" and not firmware:
  fail("UEFI firmware (OVMF_CODE.fd) not found; install OVMF to boot the x86_64 EFI image.")
 if arch!="x86_64": fail("QEMU boot arguments for "+arch+" are not yet configured; image validation completed.")
 cmd=[qemu,"-machine","q35","-smp","4","-m",memory,"-drive",
      "file="+str(image)+",format=raw,if=none,id=drv0","-device","nvme,drive=drv0"]
 if firmware: cmd += ["-bios",str(firmware)]
 log("Starting QEMU with the same raw NVMe + UEFI style used by the traditional build.")
 log("Close the VM window or terminate QEMU to return.")
 subprocess.run(cmd,check=False)
def main():
 p=argparse.ArgumentParser(description="Artifact-first TRION image builder")
 p.add_argument("--arch",default=os.getenv("ARCH","x86_64")); p.add_argument("--config",default=os.getenv("CONFIG_NAME","desktop"))
 p.add_argument("--list-configs",action="store_true"); p.add_argument("--no-rebuild-changed",action="store_true")
 p.add_argument("--qemu",action="store_true"); p.add_argument("--memory",default="2048")
 a=p.parse_args()
 if a.list_configs: print("\n".join(configs(a.arch))); return
 cfg=config_file(a.arch,a.config); installer=BUILD/"fstools"/"bin"/"redox_installer"
 acquire(installer)
 if not (ROOT/"repo").exists(): fail("Existing repo/ package repository is missing; refusing to trigger Cookbook/full compilation.")
 head=git_output("rev-parse","HEAD")
 state=json.loads(CACHE.read_text()) if CACHE.is_file() else None
 changed=changed_recipes(state.get("source_revision","") if state else "")
 if changed and a.no_rebuild_changed: fail("Changed recipe sources: "+", ".join(changed))
 if changed:
  make=shutil.which("make")
  if not make: fail("Changed recipes found but GNU make is unavailable: "+", ".join(changed))
  for recipe in changed:
   log("Rebuilding changed recipe only: "+recipe)
   subprocess.run([make,"-C",str(BUILD),"ARCH="+a.arch,"CONFIG_NAME="+a.config,"PODMAN_BUILD=0","r."+recipe],check=True)
 out=BUILD/"build"/a.arch/a.config
 log("Using existing Redox installer and package repository; Podman is bypassed.")
 log("Backend C selected: preserve native RedoxFS/bootloader semantics until an independent assembler is benchmarked faster.")
 if not cached_outputs_valid(state,a.arch,a.config,cfg,installer,out,head):
  build_one(installer,cfg,out/"harddrive.img",a.arch,a.config,False)
  build_one(installer,cfg,out/"redox-live.iso",a.arch,a.config,True)
 else: log("Verified cached images match the configuration, installer and source revision; reusing them.")
 artifacts={}
 for name in ("harddrive.img","redox-live.iso"):
  f=out/name
  if not f.is_file() or not f.stat().st_size: fail("Output missing or empty: "+str(f))
  artifacts[name]={"size":f.stat().st_size,"sha256":digest(f)}
 (out/"SHA256SUMS").write_text("".join(v["sha256"]+"  "+k+"\n" for k,v in artifacts.items()))
 CACHE.parent.mkdir(parents=True,exist_ok=True)
 state={"arch":a.arch,"config":a.config,"config_sha256":digest(cfg),"installer_sha256":digest(installer),
        "source_revision":head,"artifacts":artifacts,"backend":"redox_installer"}
 CACHE.write_text(json.dumps(state,indent=2)+"\n")
 log("Image generation/cache verification and SHA-256 validation succeeded.")
 for k,v in artifacts.items(): log(k+": "+str(v["size"])+" bytes; sha256="+v["sha256"])
 if a.qemu: qemu_boot(a.arch,out/"redox-live.iso",a.memory)
if __name__=="__main__":
 try: main()
 except subprocess.CalledProcessError as e: fail("Command failed with exit code "+str(e.returncode)+": "+str(e.cmd))
