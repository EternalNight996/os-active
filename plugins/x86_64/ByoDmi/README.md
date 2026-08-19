# How To Run  
## Prerequisites  
* A 64-bit Linux PC
* Administrator permission

## Step
* Add execute attribute  
  `chmod +x ByoDmi `
* Print help  
  `sudo ./ByoDmi -h `
* Print Version  
  `sudo ./ByoDmi -v `
* View Smbios information  
  `sudo ./ByoDmi -smbiosinfo`
* View type 0 all information    
  `sudo ./ByoDmi -viewall 0 `
* View type 0 number 0 information    
  `sudo ./ByoDmi -view 0 0 `
* Modify type 1  Product Name:  
  `sudo ./ByoDmi -type 1 0 5 ByoSoft`
* Modify type 1  UUID:  
  `sudo ./ByoDmi -type 1 0 8 11 22 33 44 55 66 77 88 99 00 11 22 33 44 55 66`  
* Lock all flash region  
  `sudo ./ByoDmi -lock`  
* Unlock all flash region  
  `sudo ./ByoDmi -unlock`  

 
## Notes
-  Must execute tool with administrator permission
-  when Tool not work, must make sure the platform support it 

-  We do not recommend set ' and " as input,it may cause many errors.If you insist,See Notes below  
-  when want to input special character like space,~,`,#,$,&,*,!,and so on.See Notes below  
-  Common special characters can be used with "" or '' to contain the whole input.  
-  If it still doesn't work.  
-  Windows use cmd instead of powershell,use "" to contain every special character(!!Every);Like a"\""\"
-  Linux can use \ before special characters;
-  Shell can use ^ before special characters;
-  All input process not tools logic.The system make it.You can search it on internet

## ReleaseNotes 
- V1.33  
  Fix update type 0 failed error  

- V1.32  
  Align error messages print  
  Fix can't update type 11 error  
  Fix dead loop error when viewall  

- V1.31  
  Fix type 4 print error  
  Fix type 12 view offset not show error  

- V1.30  
  Align print messages  
  Type0 rom size change to print exact real size  

- V1.29  
  support print all smbios type which platform installed  

- V1.28  
  Change type 17 size print error  
  Fix type 4,15,17 -view offset print error  
  Update copyright  

- V1.27  
  Fix type 3,13 print error  

- V1.26  
  Fix type 1,4,9 print error  
  Add type 13 insatlled languages print  
  Align lock status print  

- V1.25  
  Align tools check password result print  
  Update type 9,17 print messages  

- V1.24  
  Update some type with latest spec  
  -view offset show single offset print  
  -type add check input whether enough  

- V1.23  
  Add 0x before hex data print  
  Update type 17 to support latest spec  
  Check return status of write flash  

- V1.22  
  Different tools not share same lock status  

- V1.21  
  Will print null if smbios string is empty  

- V1.20  
  Align and correct printf messages 

- V1.19  
  Report version messages when check bios version fail  
  ByDmi fix copymem size too big  

- V1.18  
  Update CopyRights  
  type 11 starts from offset 1  
  Align type print messages  

- V1.17  
  Fix type 0,4,9,13,127 view error  
  Allow update type 4 offset 4,7  
  View have same format and will report error when input error  

- V1.16  
  Reorganize ByoDmi and its -view offset function  
  Change help messages format  
  Change type1 uuid print error  
 
- V1.15  
  Remove print typehandle when directly used  
  change type 9 device/function number print messages  
  change report messages to follow byoflash  
  
- V1.14  
  Report error when input error  
  add -view offset to follow shell byodmi  
  gets smbios not smbios3 address  

- V1.13  
  update type 4,9,13,17,20  
  update get version and check wrong input  

- V1.12  
  Add view type 1 sub-item with offset feature  
  Fix type 7 Maximum size and Installed size display issue  

- V1.11  
  Correct and alignment print message  

- V1.10  
  Update Copyright information  

- V1.9  
  Fix view type 3 SKU number issue  

- V1.8  
  Update print help message  
  Fix segment fault if not argument input issue  

- V1.7  
  Update for check BIOS related tool version  
  
- V1.6  
  Add Lock status check  
  Add Password check  
  Fix type 17 display issue  
  Correct type 16 display issue  
  Remove type 4 help message   
- 1.5 
  *Correct Type 16 display message
  *Fix Type 11 update string issue
- V1.4  
  *Fix SMBIOS handle larger than DMI_STRUCT_MAX issue*  
-  V1.3  
  *Fix Set DMI use wrong buffer size issue*  
- V1.2  
  *Fix some bugs*  
  *Update SMI communicate buffer and send SMI command register*  
