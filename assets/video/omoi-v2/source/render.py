from pathlib import Path
import math, json, subprocess, sys
import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy.io import wavfile

ROOT=Path(__file__).resolve().parents[1]
W,H,FPS,DUR=1920,1080,30,40
P={'night':'#120c24','blue':'#22204a','mid':'#33407a','light':'#8fa0dd','ink':'#cfd6f4','paper':'#f3efe4','pink':'#f08868','gold':'#f4cf8a','mint':'#a6dcc4'}
FONT='/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf'
cuts=[0,3,7,9,13,17,21,25,29,32,35,40]
images={p.stem:Image.open(p).convert('RGB') for p in (ROOT/'capture').glob('*.png')}

def smooth(p):
 p=max(0,min(1,p));return p*p*(3-2*p)
def pixel_text(im,xy,text,size=16,col='ink',anchor='la'):
 f=ImageFont.truetype(FONT,size); box=f.getbbox(text); mask=Image.new('1',(max(1,math.ceil(f.getlength(text))+2),size+8));ImageDraw.Draw(mask).text((0,-box[1]),text,font=f,fill=1)
 bbox=mask.getbbox();mask=mask.crop(bbox);x,y=xy
 if anchor=='center':x-=mask.width//2
 assert 0<=x and 0<=y and x+mask.width<=im.width and y+mask.height<=im.height,(text,xy,mask.size)
 im.paste(P.get(col,col),(int(x),int(y)),mask)
def world(t,ending=False):
 im=Image.new('RGB',(640,360),P['night']);d=ImageDraw.Draw(im)
 for y in range(130,275):
  for x in range((y%2)*2,640,4):
   if (y-130)/145>((x*17+y*13)%16)/16:d.point((x,y),fill=P['blue'])
 for x,y in [(46,90),(100,128),(184,76),(262,117),(370,80),(464,112),(562,61),(601,155)]:
  d.point((x,y),fill=P['ink'] if int(t*1.2+x)%3 else P['light'])
  if x in (46,562):d.line((x-1,y,x+1,y),fill=P['light'])
 # Quiet city silhouettes, not stock footage.
 for i in range(18):
  x=i*39-12; hh=30+(i*29)%62;d.rectangle((x,252-hh,x+26,256),fill=P['blue'])
  for wy in range(260-hh,245,10):
   for wx in range(x+5,x+24,8):
    if (wx+wy+i)%3==0:d.rectangle((wx,wy,wx+2,wy+3),fill=P['mid'])
 d.rectangle((0,257,639,359),fill=P['night']);d.line((0,280,639,280),fill=P['light']);d.line((0,290,639,290),fill=P['mid'])
 for x in range(-30,650,22):d.line((x,291,x+12,291),fill=P['mid'])
 # A miniature train gives the synthetic search title a visual memory.
 xx=int(100+105*smooth(t/3)) if not ending else int(205+55*smooth(t/2))
 for car in range(3):
  x=xx+car*76;d.rectangle((x,236,x+70,273),fill=P['mid']);d.rectangle((x+3,239,x+67,249),fill=P['light']);d.line((x,261,x+70,261),fill=P['pink'])
  for j in range(4):d.rectangle((x+7+j*15,242,x+16+j*15,252),fill=P['gold'])
  for dx in (13,56):d.rectangle((x+dx,274,x+dx+7,279),fill=P['night']);d.line((x+dx,279,x+dx+7,279),fill=P['light'])
  if car<2:d.line((x+70,266,x+76,266),fill=P['light'])
 if ending:
  pixel_text(im,(320,72),'omoi',57,'paper','center')
  pixel_text(im,(320,324),'APP CAPTURE / SYNTHETIC DATA',11,'light','center')
 else:
  pixel_text(im,(38,32),'OMOI',12,'pink');pixel_text(im,(38,96),'Find it again',29,'paper')
 return im.resize((W,H),Image.Resampling.NEAREST)

def stage(title,section,light=False):
 im=Image.new('RGB',(640,360),P['paper'] if light else P['night']);d=ImageDraw.Draw(im)
 ink='night' if light else 'ink';edge='mid' if light else 'light'
 pixel_text(im,(22,18),'OMOI / UNSAVED PREVIEW' if 'UNSAVED' in section else 'OMOI',10,'mid' if light else 'pink')
 pixel_text(im,(22,39),title,22,ink)
 d.rectangle((15,76,624,329),outline=P[edge]);d.rectangle((17,78,622,327),outline=P['mid'])
 pixel_text(im,(22,341),'APP CAPTURE',9,'mid' if light else 'light')
 pixel_text(im,(438,341),'SYNTHETIC DATA',9,'mid' if light else 'light')
 return im.resize((W,H),Image.Resampling.NEAREST)

def capture(im,key,box=None,zoom=1,focus=(.5,.5)):
 src=images[key]
 if box:src=src.crop(box)
 # Camera movement never changes UI content. Whole captures are fitted; detail crops fill the viewport.
 area=(57,243,1863,975); aw,ah=area[2]-area[0],area[3]-area[1]
 scale=min(aw/src.width,ah/src.height)*zoom
 sw,sh=round(src.width*scale),round(src.height*scale)
 src=src.resize((sw,sh),Image.Resampling.LANCZOS)
 tile=Image.new('RGB',(aw,ah),P['night'])
 x=round((aw-sw)*focus[0]);y=round((ah-sh)*focus[1]);tile.paste(src,(x,y));im.paste(tile,area[:2])

def frame(t):
 s=next(i for i in range(len(cuts)-1) if cuts[i]<=t<cuts[i+1]);u=t-cuts[s]
 if s==0:return world(u)
 if s==10:return world(u,True)
 if s==1:
  im=stage('Browse','01  CATALOGUE');capture(im,'03-history',(0,0,1145,510),zoom=1+.025*smooth(u/4),focus=(.5,0))
 elif s==2:
  im=stage('Search','02  SEARCH');capture(im,'04-search-input',(245,90,1130,425),zoom=1.0)
 elif s==3:
  im=stage('Find','02  FOUND')
  # A second, closer cut makes the result title legible on a phone.
  capture(im,'05-search-result',(248,184,1145,600) if u<2 else (248,400,940,535),zoom=1+.02*smooth((u%2)/2),focus=(.4,.4))
 elif s==4:
  im=stage('Inspect','03  DETAILS');capture(im,'06-details',(775,370,1155,515),zoom=1+.02*smooth(u/4),focus=(.5,.5))
 elif s==5:
  im=stage('Check openings','03  EVIDENCE');capture(im,'07-opening-evidence',(770,555,1155,730),zoom=1)
 elif s==6:
  im=stage('Tag','04  UNSAVED EDITOR PREVIEW');capture(im,'08-edit-tags',(780,475,1160,650),zoom=1.0,focus=(.5,.5))
 elif s==7:
  im=stage('Choose sources','05  SOURCES');capture(im,'02-confirmation',(250,180,920,560),zoom=1+.04*smooth(u/4))
 elif s==8:
  im=stage('Switch themes','06  APPEARANCE');capture(im,'09-appearance',zoom=1+.025*smooth(u/3))
 elif s==9:
  im=stage('Keep browsing','06  CONFLUENCE',True);capture(im,'10-confluence',(0,0,1145,505),zoom=1+.025*smooth(u/3),focus=(.5,0))
 return im

def audio():
 sr=48000;n=sr*DUR;out=np.zeros((n,2),np.float64);rng=np.random.default_rng(41)
 def tone(start,dur,midi,amp=.12,pan=0,kind='bell'):
  a=int(start*sr);length=min(int(dur*sr),n-a)
  if length<=0:return
  tt=np.arange(length)/sr;fr=440*2**((midi-69)/12)
  if kind=='bass':sig=np.sin(2*np.pi*fr*tt)+.18*np.sin(4*np.pi*fr*tt);env=np.minimum(tt/.025,1)*np.exp(-tt*2.7)
  else:sig=np.sin(2*np.pi*fr*tt+1.25*np.sin(2*np.pi*fr*2*tt)*np.exp(-tt*6));env=np.minimum(tt/.008,1)*np.exp(-tt*4.0)
  env*=np.minimum(np.maximum(dur-tt,0)/.10,1);sig*=env*amp
  out[a:a+length,0]+=sig*math.sqrt((1-pan)/2);out[a:a+length,1]+=sig*math.sqrt((1+pan)/2)
 roots=[48,44,51,46]
 melody=[72,79,75,74,72,67,70,75]
 for bar in range(18):
  start=bar*2;root=roots[(bar//2)%4]
  for off in (0,1):tone(start+off,.8,root,.14,0,'bass')
  for j in range(4):tone(start+j*.5,.75,root+12+[0,7,12,10][j],.065,(-.35 if j%2 else .35))
  if bar%2==0:
   for j in range(4):tone(start+j*.5,.7,melody[(bar+j)%8],.09,0)
  # Low, restrained pulse and a short high-frequency brushed click.
  for off in (0,1):
   a=int((start+off)*sr);L=int(.14*sr);tt=np.arange(L)/sr;sig=.08*np.sin(2*np.pi*(48*tt+4*(1-np.exp(-tt*35))))*np.exp(-tt*32);out[a:a+L]+=sig[:,None]
  for off in (.5,1.5):
   a=int((start+off)*sr);L=int(.04*sr);tt=np.arange(L)/sr;v=rng.normal(0,.007,L)*np.exp(-tt*100);out[a:a+L]+=v[:,None]
 for mi in (48,60,67,72):tone(36,3.8,mi,.11,0,'bell')
 out*=np.minimum(np.arange(n)/sr/.15,1)[:,None];fade=np.minimum((DUR-np.arange(n)/sr)/1.5,1);out*=fade[:,None]
 out*=.85/max(1,np.max(np.abs(out)));wavfile.write(ROOT/'score.wav',sr,np.int16(np.clip(out,-1,1)*32767))

def stills():
 stamps=[1.8,5,8,11,15,19,23,27,30.5,33.5,38]
 thumbs=[]
 for i,t in enumerate(stamps):
  im=frame(t);im.save(ROOT/'qa'/f'scene-{i+1:02d}.png');thumbs.append(im.resize((640,360)))
 sheet=Image.new('RGB',(1920,4*410),P['night']);d=ImageDraw.Draw(sheet);f=ImageFont.truetype(FONT,24)
 for i,(im,t) in enumerate(zip(thumbs,stamps)):
  x=(i%3)*640;y=(i//3)*410;sheet.paste(im,(x,y));d.text((x+16,y+370),f'{i+1:02d} / {t:04.1f}s',font=f,fill=P['ink'])
 sheet.save(ROOT/'Omoi — A way back — Storyboard.jpg',quality=94)

def render():
 audio();stills()
 args=['ffmpeg','-hide_banner','-loglevel','error','-y','-f','rawvideo','-pix_fmt','rgb24','-s',f'{W}x{H}','-r',str(FPS),'-i','-','-an','-c:v','libx264','-preset','fast','-crf','16','-pix_fmt','yuv420p','-threads','3',str(ROOT/'silent-master.mp4')]
 proc=subprocess.Popen(args,stdin=subprocess.PIPE)
 for i in range(DUR*FPS):
  proc.stdin.write(frame(i/FPS).tobytes())
  if i%150==0:print(f'{i}/{DUR*FPS}',flush=True)
 proc.stdin.close();assert proc.wait()==0
 for name,crf in [('Omoi — A way back — Master.mp4',16),('Omoi — A way back.mp4',24)]:
  subprocess.run(['ffmpeg','-hide_banner','-loglevel','error','-y','-i',str(ROOT/'silent-master.mp4'),'-i',str(ROOT/'score.wav'),'-c:v','copy' if crf==16 else 'libx264',* ([] if crf==16 else ['-preset','slow','-crf',str(crf),'-threads','3']),'-c:a','aac','-b:a','192k','-af','loudnorm=I=-16:TP=-1.5:LRA=9','-movflags','+faststart','-shortest',str(ROOT/name)],check=True)
 print('RENDER COMPLETE',flush=True)

if __name__=='__main__':
 if len(sys.argv)>1 and sys.argv[1]=='stills':stills()
 else:render()
