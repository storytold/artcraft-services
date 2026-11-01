/**
 * Cryptographic hashes and byte sizes for published Linux package assets (.deb, .rpm, and .pkg.tar.zst).
 * Strictly matched against official storytold release assets and SHA256SUMS.txt.
 */
export type LinuxPackageAssetMeta = {
  size: number;
  sha256: string;
  installedSize: number;
};

export const LINUX_PACKAGE_METADATA: Record<string, LinuxPackageAssetMeta> = {
  "artcraft-launcher-0.11.1-linux-aarch64.deb": {
    size: 13166808,
    sha256: "89e3eadc69667dc117b5d7950c14311e28c4732f2691fec3b4ac9453eab6861b",
    installedSize: 25074,
  },
  "artcraft-launcher-0.11.1-linux-aarch64.rpm": {
    size: 13043298,
    sha256: "912123716ece88e1b808c34873cfcc3871d086ec40d79bab258814d5e92a823f",
    installedSize: 25434431,
  },
  "artcraft-launcher-0.11.1-linux-riscv64.deb": {
    size: 13616060,
    sha256: "0636f01916d58e7014d3f9600c7c7a185197f1ac1828880f7f63689d0ea3113e",
    installedSize: 25929,
  },
  "artcraft-launcher-0.11.1-linux-riscv64.rpm": {
    size: 13592087,
    sha256: "a172ea550f528200e2ec444353d049982a962f8347e6907122c580ed835fc53a",
    installedSize: 26504570,
  },
  "artcraft-launcher-0.11.1-linux-x86_64.deb": {
    size: 13710590,
    sha256: "b1d4b4e93612e0a5d3f1f86452b784cfbdf14295eeb294ba9348a18d149034eb",
    installedSize: 26109,
  },
  "artcraft-launcher-0.11.1-linux-x86_64.rpm": {
    size: 13629087,
    sha256: "a86b7e9e709d7111f6b3a9d77a5cf3be8a7f0c8ee8b7486dfa42423787014b41",
    installedSize: 26576720,
  },
  "cadcraft-0.15.1-linux-aarch64.deb": {
    size: 70865890,
    sha256: "41cdc9a7ffbfec56759ea890d60779872c7b152c17244be6c338125b7a00a589",
    installedSize: 134950,
  },
  "cadcraft-0.15.1-linux-aarch64.rpm": {
    size: 70294613,
    sha256: "c5b84441bc05da0916feeec221e234a4f2738145994e27e1b1f757b0463215f4",
    installedSize: 137074495,
  },
  "cadcraft-0.15.1-linux-riscv64.deb": {
    size: 71556780,
    sha256: "6a9c50068bbdcd7c1355e97ea42fafabe38e3762bcc8244e25310167a027d62f",
    installedSize: 136265,
  },
  "cadcraft-0.15.1-linux-riscv64.rpm": {
    size: 71285171,
    sha256: "617303a3bd9d4cbf431fb95335175ef5cdcf413cb7d4777a18ede537482a4917",
    installedSize: 139006083,
  },
  "cadcraft-0.15.1-linux-x86_64.deb": {
    size: 72938636,
    sha256: "8709398e54dd0d9a45524f81bdaf776aa6fe5e32db0b6f9493d94b8e444c9135",
    installedSize: 138897,
  },
  "cadcraft-0.15.1-linux-x86_64.rpm": {
    size: 72490148,
    sha256: "fc8e3556784ab5026119693538274cbe0ee95310d641212b3cdfd4a5a8f4c03b",
    installedSize: 141355789,
  },
  "deckcraft-0.13.1-linux-aarch64.deb": {
    size: 102879740,
    sha256: "32795e255307e968e73720ac355c79f523403f185db1939b5fbf331ab55a9675",
    installedSize: 195914,
  },
  "deckcraft-0.13.1-linux-aarch64.rpm": {
    size: 102340520,
    sha256: "61bba910372ad3e0c77e2f4924351e36465f041cc5622d3dbedfb538dfe243b9",
    installedSize: 199564014,
  },
  "deckcraft-0.13.1-linux-riscv64.deb": {
    size: 103662202,
    sha256: "dec592f8fd4381a4271d4beaf7f8812df314d6e401a9b768cbb512f90405af49",
    installedSize: 197404,
  },
  "deckcraft-0.13.1-linux-riscv64.rpm": {
    size: 103387779,
    sha256: "143fbb328ad657794b13a59bc71242a5e7b04ec99890180eccc34d5cb074b229",
    installedSize: 201606169,
  },
  "deckcraft-0.13.1-linux-x86_64.deb": {
    size: 104761432,
    sha256: "f26dc05adfd197b53003c9bd5241e4caab8bf30f4bd40682ef9dffa6f78f85d5",
    installedSize: 199497,
  },
  "deckcraft-0.13.1-linux-x86_64.rpm": {
    size: 104319498,
    sha256: "16207e940f78f46c5e5495866a498201fef92de786d98c82e6198ba6831d9a00",
    installedSize: 203423021,
  },
  "designcraft-0.15.2-linux-aarch64.deb": {
    size: 122027154,
    sha256: "3b7c4dc5fcd6d87b7da500cdc4e6f1675f7fbd1f64128c0c3f3c6c562ac47ccb",
    installedSize: 232376,
  },
  "designcraft-0.15.2-linux-aarch64.rpm": {
    size: 121250464,
    sha256: "9e33a3f406e29607c7f0a1e269d0ac464007b23a18e5362e91a4f6ef648f4d25",
    installedSize: 236438405,
  },
  "designcraft-0.15.2-linux-riscv64.deb": {
    size: 123216784,
    sha256: "429bd2b8bd22d9c44cd56d2a7d56463b681554332d35949f65cd345bbfbdc6af",
    installedSize: 234641,
  },
  "designcraft-0.15.2-linux-riscv64.rpm": {
    size: 122831612,
    sha256: "33b9d373aa1f15ae02dea804d2f808037a98cd84a0efa9ee7738528970fddacc",
    installedSize: 239521643,
  },
  "designcraft-0.15.2-linux-x86_64.deb": {
    size: 125288114,
    sha256: "02dd8ff663be60deed416cbf9081ff21ef63f619256f68f0c147ecf7aa79a202",
    installedSize: 238586,
  },
  "designcraft-0.15.2-linux-x86_64.rpm": {
    size: 124660802,
    sha256: "89cbc206809f1b93533e4da81004c13f5dccf6a2e6c91e4acc93ba0b32653230",
    installedSize: 243088564,
  },
  "effectcraft-0.19.0-linux-aarch64.deb": {
    size: 75552968,
    sha256: "b4ae8931f126e0b04308dcb343dd462528851fe42df7f9435dde4014c285ecd2",
    installedSize: 143875,
  },
  "effectcraft-0.19.0-linux-aarch64.rpm": {
    size: 74801818,
    sha256: "bf5df439034a676788f11e52859c587777189ee6ed8ee28465c5b1d6bee36916",
    installedSize: 145863545,
  },
  "effectcraft-0.19.0-linux-riscv64.deb": {
    size: 76651644,
    sha256: "75ea5edb0e257b33e29b8ad6c7a4af39b98938991987bd2af72a82342b2d2695",
    installedSize: 145967,
  },
  "effectcraft-0.19.0-linux-riscv64.rpm": {
    size: 76466186,
    sha256: "311de80c70710f313a87ec5ddc7465df39de94fe234b1bb1616b40c7af8ea875",
    installedSize: 149109063,
  },
  "effectcraft-0.19.0-linux-x86_64.deb": {
    size: 79317050,
    sha256: "4a9051d1aa001fcd74bffe41c096fe9f12256b049c4461b1ec9f3bb4f8dd5a9f",
    installedSize: 151043,
  },
  "effectcraft-0.19.0-linux-x86_64.rpm": {
    size: 78809275,
    sha256: "704e6682016573cc02796d9659e080272a77c74e854f4dc31965702e3d37b814",
    installedSize: 153678086,
  },
  "figcraft-0.13.1-linux-aarch64.deb": {
    size: 100442836,
    sha256: "f2dfb981e2ed14eba11c6fe96b15b59a007ab5463c547bb9782465a658944ff8",
    installedSize: 191273,
  },
  "figcraft-0.13.1-linux-aarch64.rpm": {
    size: 99762783,
    sha256: "dd7d9b891fe1a9c6287fd3d344460077d05327de7c51f426dcac27aa2382dd37",
    installedSize: 194537427,
  },
  "figcraft-0.13.1-linux-riscv64.deb": {
    size: 100748216,
    sha256: "fb85b070cc345b0e409c9510e5120177dfa70c78de7876fba93404c8875b764a",
    installedSize: 191855,
  },
  "figcraft-0.13.1-linux-riscv64.rpm": {
    size: 100445380,
    sha256: "c24007d73e1a1442f584bc95d6abac32bcc60555884b3317ed11c02a413cdda1",
    installedSize: 195868491,
  },
  "figcraft-0.13.1-linux-x86_64.deb": {
    size: 103017754,
    sha256: "b288004a9d9755487c874d5d3e865ae4d7350cbc196cf4ec13b630d698afeea7",
    installedSize: 196176,
  },
  "figcraft-0.13.1-linux-x86_64.rpm": {
    size: 102507244,
    sha256: "b870423ce109fe040484736e61e54d9bc4d601b600f5b1b04a3295bcaeb2c40a",
    installedSize: 199889126,
  },
  "filmcraft-0.15.0-linux-aarch64.deb": {
    size: 105127818,
    sha256: "f227aac558901da66b33957609d3ca8475a91f1f217f1bc7570fa7abccae6b74",
    installedSize: 200195,
  },
  "filmcraft-0.15.0-linux-aarch64.rpm": {
    size: 104339809,
    sha256: "5301d45c1b34aa1976cbe3620eb30a0c97cf78c9d97cf42eaf272fb16bc69c6f",
    installedSize: 203462628,
  },
  "filmcraft-0.15.0-linux-riscv64.deb": {
    size: 105607396,
    sha256: "fab0a29fa173edee957760c02adb1f10cdf1476d3ac5b42053b9f5c9ccedff7c",
    installedSize: 201108,
  },
  "filmcraft-0.15.0-linux-riscv64.rpm": {
    size: 105154984,
    sha256: "967f26c5b68d0608a7fe235105adfd8f8aec288eee2a52eb8112c7098dd4c0a0",
    installedSize: 205052219,
  },
  "filmcraft-0.15.0-linux-x86_64.deb": {
    size: 107248810,
    sha256: "18d8d40d080b0ff6264234d5d610d290bfa0fcfdc50f6e0cea59ae31ef81fc4b",
    installedSize: 204234,
  },
  "filmcraft-0.15.0-linux-x86_64.rpm": {
    size: 106617320,
    sha256: "f5b08905ee94bbcb0044c2da63ff8503fd3ebc22ecc83a37cb4b047a90b791aa",
    installedSize: 207903774,
  },
  "gridcraft-0.14.2-linux-aarch64.deb": {
    size: 48494174,
    sha256: "27f6c9e4be9653687b9b064a003b3c4ef0e6abcafc0cb9fa5cb620ef86f6b893",
    installedSize: 92347,
  },
  "gridcraft-0.14.2-linux-aarch64.rpm": {
    size: 48160140,
    sha256: "0aecb26427c7e13383bfb827dc7d0565700500ae4aa94c87545458a7bd4d38ef",
    installedSize: 93912273,
  },
  "gridcraft-0.14.2-linux-riscv64.deb": {
    size: 48596458,
    sha256: "227d11d9115bbbac62598355281952d59a985ee0a425524f7ae3b8f795c92152",
    installedSize: 92542,
  },
  "gridcraft-0.14.2-linux-riscv64.rpm": {
    size: 48451692,
    sha256: "d7544d75c47c5ab54d4e697cd0813bae12c7f43d6aaa5fcee272e9fe81fb8bad",
    installedSize: 94480799,
  },
  "gridcraft-0.14.2-linux-x86_64.deb": {
    size: 49752594,
    sha256: "dde3b1fd4deb29bbdc0f2105e8f81d1f8d674baaabfe063ea4ed13142fa740e5",
    installedSize: 94744,
  },
  "gridcraft-0.14.2-linux-x86_64.rpm": {
    size: 49500378,
    sha256: "07a87730245fac4959ac113e111ae73367aaff3271786cf9a60e0ab3cf58ea4d",
    installedSize: 96525737,
  },
  "lightcraft-0.16.0-linux-aarch64.deb": {
    size: 116448826,
    sha256: "86d16bc8c013fb146311df19f5d0e27ea75ff95f6652778ffd1b21d4d5e6043a",
    installedSize: 221753,
  },
  "lightcraft-0.16.0-linux-aarch64.rpm": {
    size: 115639594,
    sha256: "70902b0a730851f872d020bff4879106ab51059ffc41552551d6d6aa55af6849",
    installedSize: 225497208,
  },
  "lightcraft-0.16.0-linux-riscv64.deb": {
    size: 117316878,
    sha256: "37851f05fbf6ccf52a79b50428fa4dacae79b11bcf95e6c109263cee6f386631",
    installedSize: 223406,
  },
  "lightcraft-0.16.0-linux-riscv64.rpm": {
    size: 116862921,
    sha256: "dc15eca17edd7e4b4faecb34735bb1261937c2cfb66d77eb000d42fa8f792c8a",
    installedSize: 227882696,
  },
  "lightcraft-0.16.0-linux-x86_64.deb": {
    size: 120837496,
    sha256: "76a94847aeb510c6453605dd37cf86cdaa89a4b3f207418bc9dcf57016d72dd0",
    installedSize: 230110,
  },
  "lightcraft-0.16.0-linux-x86_64.rpm": {
    size: 120136258,
    sha256: "95478e75059728a072de512096b9d5d9d509f5bb175b6f5072777b41aacf3276",
    installedSize: 234265703,
  },
  "pdfcraft-0.16.1-linux-aarch64.deb": {
    size: 109221366,
    sha256: "7c90af4c6af71da3efb017b697163267c5930e24e4d6b041edb8be0c1e23c31e",
    installedSize: 207990,
  },
  "pdfcraft-0.16.1-linux-aarch64.rpm": {
    size: 108543068,
    sha256: "9034a6ff6e40443f700f869c357bc9a5dc4d859fe87fc4a5dcf3759d57750810",
    installedSize: 211658983,
  },
  "pdfcraft-0.16.1-linux-riscv64.deb": {
    size: 109483726,
    sha256: "853e185e3c665c6da3c9d6973a121b2b0bb745c0aaab5880b9efe4438517a826",
    installedSize: 208490,
  },
  "pdfcraft-0.16.1-linux-riscv64.rpm": {
    size: 109167000,
    sha256: "8f1e9fa6b3da20df374d12db2bf895340530a48cd72c955e9bcfe9a458976c7e",
    installedSize: 212875650,
  },
  "pdfcraft-0.16.1-linux-x86_64.deb": {
    size: 112137992,
    sha256: "fb71f9b366214c8b0dafe01afba80d5280d554a7683ab7756f601a170668823b",
    installedSize: 213544,
  },
  "pdfcraft-0.16.1-linux-x86_64.rpm": {
    size: 111638587,
    sha256: "b5d30dde0d5dea634af7a8f0a7bb1f81b75861ae7338c5b4c76a9b1da66ef2d3",
    installedSize: 217695245,
  },
  "photocraft-0.18.0-linux-aarch64.deb": {
    size: 122276372,
    sha256: "7c47c5aa193fd85f992d46b47e24b2c0e0dae5079ea4c3bde940b44a2f85e388",
    installedSize: 232851,
  },
  "photocraft-0.18.0-linux-aarch64.rpm": {
    size: 121256777,
    sha256: "e9967e94cdef995949f00e8f98ecc8012fcbe869cfc60918c37a130dd53d2e87",
    installedSize: 236450715,
  },
  "photocraft-0.18.0-linux-riscv64.deb": {
    size: 123269740,
    sha256: "d1c8b33b0863d80071213dccd8fe81532cc5928a2eee09d4de385237eeb63cb0",
    installedSize: 234742,
  },
  "photocraft-0.18.0-linux-riscv64.rpm": {
    size: 122711179,
    sha256: "dcec0e5b7596c22d123282d4986b73b4bd866df43a58bd5caf7232fd795982bb",
    installedSize: 239286799,
  },
  "photocraft-0.18.0-linux-x86_64.deb": {
    size: 126144548,
    sha256: "8d8905f3cc516270b0c66481ef79ae6d7fa33947aae92402137be47834c33443",
    installedSize: 240217,
  },
  "photocraft-0.18.0-linux-x86_64.rpm": {
    size: 125330995,
    sha256: "275659683f2ee70cfa6c4d3d2d61acd86fa17da632b90db88e0b9e738bba5082",
    installedSize: 244395440,
  },
  "soundcraft-0.15.0-linux-aarch64.deb": {
    size: 19810362,
    sha256: "66638d83db1d09fddc2c16ed35e69ea97b68b49b57ae9dd186c8e85b8ee833c7",
    installedSize: 37725,
  },
  "soundcraft-0.15.0-linux-aarch64.rpm": {
    size: 19608551,
    sha256: "73aed7c60a7651d7a091015dc91e6bcd27f5c615276f6588a12d7babe1b8ea66",
    installedSize: 38236674,
  },
  "soundcraft-0.15.0-linux-riscv64.deb": {
    size: 19995720,
    sha256: "c568548cbc077e1652ca891b99e1e66d10a5e0d367208a2520ecda0c768c5e64",
    installedSize: 38078,
  },
  "soundcraft-0.15.0-linux-riscv64.rpm": {
    size: 19945717,
    sha256: "ba5bc45967204820610eaf3638e5b94c66baa999af6aeb6d5605399e983a9da6",
    installedSize: 38894148,
  },
  "soundcraft-0.15.0-linux-x86_64.deb": {
    size: 21060466,
    sha256: "97d523d88556c23c870cd943c4744283b734caaa4409e9b7b1076a494ef99241",
    installedSize: 40105,
  },
  "soundcraft-0.15.0-linux-x86_64.rpm": {
    size: 20914241,
    sha256: "a2eaf800fe31afcb50b09986d53054c16b12b472601bcb853d3982119bad6da0",
    installedSize: 40782770,
  },
  "vectorcraft-0.19.0-linux-aarch64.deb": {
    size: 136358770,
    sha256: "2ab63c41506d95fb77ceecd8ead9c9fe8dffd58394f0930ecbb37abad7bd7f4b",
    installedSize: 259668,
  },
  "vectorcraft-0.19.0-linux-aarch64.rpm": {
    size: 135168810,
    sha256: "97094ea6d6d816211aa0b5b3d8799b2dacfdf0356adb598a2974a129e66d93bb",
    installedSize: 263579180,
  },
  "vectorcraft-0.19.0-linux-riscv64.deb": {
    size: 137919218,
    sha256: "1163eb958ea43275ea463f21f4c79f3ed6a9cffb4da7de6b69b846994895f8a6",
    installedSize: 262639,
  },
  "vectorcraft-0.19.0-linux-riscv64.rpm": {
    size: 137301763,
    sha256: "f5a9e553eed75857fcffa482e0740adaa63344bd3e9cb03f6c3c07742f4dade7",
    installedSize: 267738438,
  },
  "vectorcraft-0.19.0-linux-x86_64.deb": {
    size: 140606758,
    sha256: "cfd8ae77214029aae63d5a5fc9adb7d7628765de5faa37dddfb658bc5980bf11",
    installedSize: 267757,
  },
  "vectorcraft-0.19.0-linux-x86_64.rpm": {
    size: 139652058,
    sha256: "b507931d82471eef60b5f8f49de74d803c6c14e9e89f75324115ea0cf348445c",
    installedSize: 272321513,
  },
  "wordcraft-0.13.2-linux-aarch64.deb": {
    size: 109021264,
    sha256: "c7c191d45e56a479194e05a4336d8c56a878fee553eb7105fc256703d981688b",
    installedSize: 207609,
  },
  "wordcraft-0.13.2-linux-aarch64.rpm": {
    size: 108278473,
    sha256: "c05494ea1021333a00b87452ce070b5120eb7578fc541afea45c05d34019314b",
    installedSize: 211143022,
  },
  "wordcraft-0.13.2-linux-riscv64.deb": {
    size: 109961620,
    sha256: "3e0157bc9d11e691f0101f6b0b878782a73980372862e5180997b10051d5d9c3",
    installedSize: 209400,
  },
  "wordcraft-0.13.2-linux-riscv64.rpm": {
    size: 109553113,
    sha256: "ef0257dfe5024fa30cf1c1307bfb4cf5896106fce9b78a99da2902ffeb761af9",
    installedSize: 213628570,
  },
  "wordcraft-0.13.2-linux-x86_64.deb": {
    size: 111456170,
    sha256: "8b4ff5c43e4d5e4a116cfd6f64262017d68fa1824d7afa994a9b66129208cc68",
    installedSize: 212246,
  },
  "wordcraft-0.13.2-linux-x86_64.rpm": {
    size: 110841140,
    sha256: "c85e3da9dd7fb098d0a1724f6ce6c96983ad5cc6c13513d2732787efab4b471b",
    installedSize: 216140223,
  },
  "artcraft-launcher-0.11.1-linux-aarch64.pkg.tar.zst": {
    size: 9674991,
    sha256: "73587503489038e92ff58232fd0cda4b1547f02a172da9164cbc6156ff7e2488",
    installedSize: 27269309,
  },
  "artcraft-launcher-0.11.1-linux-x86_64.pkg.tar.zst": {
    size: 10153348,
    sha256: "749a815a6f6b39b8bce9561634d79351f1c723fe21db1becf9103d8d578efba3",
    installedSize: 30684341,
  },
  "cadcraft-0.15.1-linux-aarch64.pkg.tar.zst": {
    size: 52413189,
    sha256: "c05799b6946dbb7adb95f192dadfd89a58051728324601feac9fe24680ac7f06",
    installedSize: 138941762,
  },
  "cadcraft-0.15.1-linux-x86_64.pkg.tar.zst": {
    size: 54279352,
    sha256: "82799c5bf1bbd8c81ad9a404cbc11037f6427758777b49722779239ca8b63029",
    installedSize: 151955818,
  },
  "deckcraft-0.13.1-linux-aarch64.pkg.tar.zst": {
    size: 75453808,
    sha256: "20cc1174bf139d0bdb78e11ab888fd77b740065f4f788414a8b6c2080f4e7b21",
    installedSize: 198928538,
  },
  "deckcraft-0.13.1-linux-x86_64.pkg.tar.zst": {
    size: 77366880,
    sha256: "684970a9808647261c9f180c1548390d6f60b9a97926d3f07b108799b2e3dc65",
    installedSize: 210294850,
  },
  "designcraft-0.15.2-linux-aarch64.pkg.tar.zst": {
    size: 116179766,
    sha256: "0ec149e9dbcd5791bdf0f3ebd5d318e7e12eb9af800e083796b1297cb6a4d8a2",
    installedSize: 222699462,
  },
  "designcraft-0.15.2-linux-x86_64.pkg.tar.zst": {
    size: 120007985,
    sha256: "217f40a83065e47f2f361b77c45e2ca0a78d3c305f57144f365550d7754a675b",
    installedSize: 240682966,
  },
  "effectcraft-0.19.0-linux-aarch64.pkg.tar.zst": {
    size: 50931930,
    sha256: "62e8cfc33931bd4a0d6c755629fca1795478a0bf919fd641f2ebafd3d0e2fffb",
    installedSize: 165289975,
  },
  "effectcraft-0.19.0-linux-x86_64.pkg.tar.zst": {
    size: 54128758,
    sha256: "1a408fa3eaaf5e8daa2aa973ba7cbdd9364951c047b1f1a1291f7aca03b25060",
    installedSize: 191343719,
  },
  "figcraft-0.13.1-linux-aarch64.pkg.tar.zst": {
    size: 74074303,
    sha256: "2c34247b4333fb5da683226fff55c550d15c8f2c390d11bb32fe43dd4f6dab05",
    installedSize: 201213173,
  },
  "figcraft-0.13.1-linux-x86_64.pkg.tar.zst": {
    size: 76562658,
    sha256: "b1c58a4fb0e81881407f8252336ce2e005a81dcbb5937bb034950d602bd2a768",
    installedSize: 216533701,
  },
  "filmcraft-0.15.0-linux-aarch64.pkg.tar.zst": {
    size: 82294862,
    sha256: "01686ba85b3457c916c62ed4f00097b0e4abb2849cfa09d2e616792be8076084",
    installedSize: 183720236,
  },
  "filmcraft-0.15.0-linux-x86_64.pkg.tar.zst": {
    size: 84148913,
    sha256: "a8a4eac00c13ef5d93357222f9df67ede65710c6086b911ae672a3a60db492b8",
    installedSize: 198809372,
  },
  "gridcraft-0.14.2-linux-aarch64.pkg.tar.zst": {
    size: 35878355,
    sha256: "25e4f0bc4f1c2343aa6a56cf0ac8bf9129decdea92196300c8486afa67c17924",
    installedSize: 95146647,
  },
  "gridcraft-0.14.2-linux-x86_64.pkg.tar.zst": {
    size: 34676019,
    sha256: "ba5f5a2726a3b2208113fe7f8a927deeddd18af9ca9250e23acafe1f06939bf7",
    installedSize: 101693079,
  },
  "lightcraft-0.16.0-linux-aarch64.pkg.tar.zst": {
    size: 88893201,
    sha256: "ce15a9c363077646d8bad3b9571ad8a67e6a7a747986ff338861fc034da5d742",
    installedSize: 220349168,
  },
  "lightcraft-0.16.0-linux-x86_64.pkg.tar.zst": {
    size: 92297878,
    sha256: "c07c4e48668f21e2f5727bdf1e204ad5fdda8dcda03a3f92fd8f9f4af90d0bd2",
    installedSize: 242435024,
  },
  "pdfcraft-0.16.1-linux-aarch64.pkg.tar.zst": {
    size: 84101213,
    sha256: "a2e09d1fb7c10093d7480a12b09592c26fba061ddaeaee193abf0507d3960091",
    installedSize: 201732541,
  },
  "pdfcraft-0.16.1-linux-x86_64.pkg.tar.zst": {
    size: 86612887,
    sha256: "66fda9a046b483cf0dbfbae8346c1d5acce1aabf9aeac720d7aabd21fda76879",
    installedSize: 218674077,
  },
  "photocraft-0.18.0-linux-aarch64.pkg.tar.zst": {
    size: 96984615,
    sha256: "ad79e07063f70f84aeebaef11879291c5b59f286b6514b14be05ea7cb345fad7",
    installedSize: 212570126,
  },
  "photocraft-0.18.0-linux-x86_64.pkg.tar.zst": {
    size: 100260814,
    sha256: "4f5ac055c5352dd678673463eae0217d4a1c39bd76fcf0fe6ad8ab8ffa2f664a",
    installedSize: 234705382,
  },
  "soundcraft-0.15.0-linux-aarch64.pkg.tar.zst": {
    size: 12057562,
    sha256: "ca6fdaf2a96ec4bcf681fa842f0cbede9b319eb011928dc205fa4539a6a16f19",
    installedSize: 43628667,
  },
  "soundcraft-0.15.0-linux-x86_64.pkg.tar.zst": {
    size: 13111599,
    sha256: "e2b88503bc90f28b260c6497ff872449b216d858caad9b8317cd14196636b8c6",
    installedSize: 49807820,
  },
  "vectorcraft-0.19.0-linux-aarch64.pkg.tar.zst": {
    size: 99412945,
    sha256: "9c96ba564fe9ccc0a768c4c6609e29f1377ddd56dc60788126b90312bafc378d",
    installedSize: 248885906,
  },
  "vectorcraft-0.19.0-linux-x86_64.pkg.tar.zst": {
    size: 103250726,
    sha256: "bf0c2098c3cacec5e8dfdf162f3923c64f982aa09bb1e0617b89661f8e39d598",
    installedSize: 273029330,
  },
  "wordcraft-0.13.2-linux-aarch64.pkg.tar.zst": {
    size: 82644991,
    sha256: "fe250f94feb4294ab46d99d5f0ec5fcab948b10470fd36bf6ff30913bf0509d7",
    installedSize: 197307172,
  },
  "wordcraft-0.13.2-linux-x86_64.pkg.tar.zst": {
    size: 84034875,
    sha256: "b201a3ff98313e481b2fee6b27352a32165d93f04d5291f1088cfa4edba96d5f",
    installedSize: 210739020,
  },
};
