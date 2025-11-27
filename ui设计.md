import React, { useState, useEffect } from 'react';
import { 
  Home, 
  Wrench, 
  Database, 
  MessageSquare, 
  FileText, 
  Settings, 
  Activity, 
  HardDrive, 
  Cpu, 
  Terminal,
  Play,
  CassetteTape
} from 'lucide-react';

export default function RetroNeuroSpec() {
  const [activeTab, setActiveTab] = useState('首页');
  const [isLoaded, setIsLoaded] = useState(false);
  
  // Simulate CRT flicker or boot sequence
  useEffect(() => {
    const timer = setTimeout(() => setIsLoaded(true), 100);
    return () => clearTimeout(timer);
  }, []);

  const menuItems = [
    { icon: Home, label: '首页' },
    { icon: Wrench, label: '工具' },
    { icon: Database, label: '记忆' },
    { icon: MessageSquare, label: '提示词' },
    { icon: FileText, label: 'AGENTS' },
    { icon: Settings, label: '设置' },
  ];

  const statusCards = [
    { 
      title: 'MCP STATUS', 
      value: 'CONNECTED', 
      color: 'bg-emerald-500', 
      icon: Activity,
      subtext: 'SIGNAL OK'
    },
    { 
      title: 'INDEX', 
      value: '[object Object]', 
      subValue: 'FILES',
      color: 'bg-amber-500', 
      icon: HardDrive,
      subtext: 'BUFFERING'
    },
    { 
      title: 'MEMORY', 
      value: '0 ITEMS', 
      color: 'bg-rose-500', 
      icon: Cpu,
      subtext: 'CAPACITY 100%'
    },
  ];

  const actionButtons = [
    { icon: FileText, label: 'AGENTS.md', color: 'bg-blue-100' },
    { icon: Wrench, label: '工具调试', color: 'bg-green-100' },
    { icon: Database, label: '记忆管理', color: 'bg-purple-100' },
    { icon: Settings, label: '设置', color: 'bg-gray-100' },
  ];

  return (
    <div className={`min-h-screen bg-[#e8e4d9] font-mono text-gray-800 p-4 md:p-8 transition-opacity duration-700 ${isLoaded ? 'opacity-100' : 'opacity-0'}`}>
      
      {/* Texture Overlay (Noise) */}
      <div className="fixed inset-0 pointer-events-none opacity-[0.03] z-50 mix-blend-multiply" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E")` }}></div>

      <div className="max-w-7xl mx-auto flex flex-col md:flex-row gap-8 h-[calc(100vh-4rem)]">
        
        {/* Left Sidebar - Cassette Spine Style */}
        <div className="w-full md:w-64 flex-shrink-0 flex flex-col gap-6">
          {/* Logo Section */}
          <div className="border-4 border-gray-800 bg-white p-4 shadow-[8px_8px_0px_0px_rgba(31,41,55,1)] relative overflow-hidden group">
            <div className="absolute top-0 right-0 w-8 h-8 bg-orange-500 transform rotate-45 translate-x-4 -translate-y-4"></div>
            <div className="flex items-center gap-3 relative z-10">
              <div className="w-10 h-10 bg-gray-900 rounded-full flex items-center justify-center border-2 border-gray-800 text-white">
                <CassetteTape size={24} />
              </div>
              <div>
                <h1 className="text-xl font-black tracking-tighter uppercase leading-none">NeuroSpec</h1>
                <p className="text-xs font-bold text-gray-500 mt-1">v0.2.0_BETA</p>
              </div>
            </div>
          </div>

          {/* Navigation Menu - Mechanical Buttons */}
          <nav className="flex-1 flex flex-col gap-3">
            {menuItems.map((item) => (
              <button
                key={item.label}
                onClick={() => setActiveTab(item.label)}
                className={`
                  flex items-center gap-4 px-4 py-3 border-2 border-gray-800 transition-all duration-100
                  ${activeTab === item.label 
                    ? 'bg-orange-500 text-white translate-x-1 translate-y-1 shadow-none font-bold' 
                    : 'bg-[#f4f1ea] hover:bg-white shadow-[4px_4px_0px_0px_rgba(31,41,55,1)] hover:-translate-y-0.5 hover:-translate-x-0.5'}
                `}
              >
                <item.icon size={20} className={activeTab === item.label ? 'stroke-[3px]' : 'stroke-2'} />
                <span className="text-lg uppercase tracking-wide">{item.label}</span>
                {activeTab === item.label && <div className="ml-auto w-2 h-2 bg-white rounded-full animate-pulse" />}
              </button>
            ))}
          </nav>

          {/* Bottom Sidebar Settings */}
          <div className="p-4 border-2 border-gray-800 bg-gray-200 shadow-[4px_4px_0px_0px_rgba(31,41,55,1)]">
            <div className="flex items-center gap-2 text-xs font-bold text-gray-600 mb-2 uppercase">
              <div className="w-3 h-3 rounded-full bg-green-500 border border-black animate-pulse"></div>
              System Online
            </div>
            <div className="w-full h-2 bg-gray-300 border border-gray-600 rounded-full overflow-hidden">
              <div className="h-full bg-gray-800 w-2/3"></div>
            </div>
          </div>
        </div>

        {/* Main Content Area - Cassette J-Card Style */}
        <div className="flex-1 border-4 border-gray-800 bg-[#fbfaf8] shadow-[12px_12px_0px_0px_rgba(31,41,55,1)] relative flex flex-col overflow-hidden">
          
          {/* Top Decorative Stripe */}
          <div className="h-4 w-full flex border-b-4 border-gray-800">
            <div className="w-1/3 bg-orange-500"></div>
            <div className="w-1/3 bg-teal-600"></div>
            <div className="w-1/3 bg-gray-800"></div>
          </div>

          <div className="p-8 flex-1 overflow-y-auto custom-scrollbar">
            
            {/* Header Section */}
            <header className="mb-10 pb-6 border-b-2 border-dashed border-gray-400">
              <div className="flex items-end justify-between mb-2">
                <div>
                  <h2 className="text-4xl font-black uppercase tracking-tight text-gray-900 mb-2">
                    NEUROSPEC <span className="text-white bg-blue-600 px-2 text-lg align-middle transform -rotate-2 inline-block border-2 border-black ml-2">v0.4.0</span>
                  </h2>
                  <p className="text-gray-500 font-bold tracking-widest text-sm flex items-center gap-2">
                    <span className="inline-block w-4 h-[2px] bg-gray-500"></span>
                    AI-POWERED DEVELOPMENT ASSISTANT
                    <span className="inline-block w-4 h-[2px] bg-gray-500"></span>
                  </p>
                </div>
                <div className="hidden md:block text-right">
                   <div className="border-2 border-gray-800 px-2 py-1 inline-block bg-gray-100 font-bold text-xs">
                     SIDE A
                   </div>
                </div>
              </div>
            </header>

            {/* Status Cards - "Sticker" Style */}
            <div className="grid grid-cols-1 md:grid-cols-3 gap-6 mb-10">
              {statusCards.map((card, idx) => (
                <div key={idx} className="relative group">
                  {/* Tape/Sticker effect */}
                  <div className="absolute -top-3 left-1/2 transform -translate-x-1/2 w-24 h-6 bg-yellow-100/50 rotate-1 border border-yellow-200/50 z-0"></div>
                  
                  <div className="relative z-10 bg-white border-2 border-gray-800 p-5 shadow-[6px_6px_0px_0px_rgba(200,200,200,1)] hover:shadow-[6px_6px_0px_0px_rgba(31,41,55,1)] transition-all duration-200 hover:-translate-y-1">
                    <div className="flex items-center gap-2 mb-3 border-b-2 border-gray-100 pb-2">
                      <div className={`w-3 h-3 rounded-full ${card.color} border border-black`}></div>
                      <span className="text-xs font-bold text-gray-400 uppercase tracking-widest">{card.title}</span>
                    </div>
                    <div className="font-bold text-xl text-gray-800 break-words leading-tight min-h-[3rem] flex items-center">
                      {card.value}
                    </div>
                    {card.subValue && (
                       <div className="font-bold text-xl text-gray-800 break-words leading-tight">{card.subValue}</div>
                    )}
                    <div className="mt-4 flex items-center justify-between text-[10px] font-bold text-gray-400 font-mono">
                      <span>{card.subtext}</span>
                      <card.icon size={14} />
                    </div>
                  </div>
                </div>
              ))}
            </div>

            {/* Action Buttons Grid */}
            <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-12">
              {actionButtons.map((btn, idx) => (
                <button 
                  key={idx}
                  className={`
                    group relative h-32 flex flex-col items-center justify-center gap-3
                    bg-white border-2 border-gray-800 
                    shadow-[4px_4px_0px_0px_rgba(31,41,55,1)] active:shadow-none active:translate-x-1 active:translate-y-1
                    transition-all duration-100
                  `}
                >
                  <div className={`
                    w-12 h-12 rounded-lg ${btn.color} border-2 border-gray-800 
                    flex items-center justify-center group-hover:scale-110 transition-transform
                  `}>
                    <btn.icon size={24} className="stroke-gray-800" />
                  </div>
                  <span className="font-bold text-sm uppercase">{btn.label}</span>
                  
                  {/* Corner screws decoration */}
                  <div className="absolute top-1 left-1 text-[8px] opacity-20">+</div>
                  <div className="absolute top-1 right-1 text-[8px] opacity-20">+</div>
                  <div className="absolute bottom-1 left-1 text-[8px] opacity-20">+</div>
                  <div className="absolute bottom-1 right-1 text-[8px] opacity-20">+</div>
                </button>
              ))}
            </div>

            {/* Terminal / Log Section */}
            <div className="border-2 border-gray-800 bg-[#1a1c20] text-green-500 p-4 font-mono text-sm shadow-[inset_0_0_20px_rgba(0,0,0,0.5)] relative overflow-hidden">
               {/* Scanline effect */}
              <div className="absolute inset-0 bg-[linear-gradient(rgba(18,16,16,0)_50%,rgba(0,0,0,0.25)_50%),linear-gradient(90deg,rgba(255,0,0,0.06),rgba(0,255,0,0.02),rgba(0,0,255,0.06))] z-10 bg-[length:100%_2px,3px_100%] pointer-events-none"></div>
              
              <div className="relative z-20 opacity-90">
                <div className="flex justify-between items-center mb-4 border-b border-gray-700 pb-2 text-xs text-gray-400">
                  <span className="flex items-center gap-2">
                    <Terminal size={12} />
                    SYSTEM_LOG
                  </span>
                  <span>E:\AI-coding-creating\neruospec</span>
                </div>
                
                <div className="space-y-2">
                  <div className="flex justify-between">
                    <span className="text-gray-500">PROJECT:</span>
                    <span className="text-white">NeuroSpec_v2</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-500">LAST_SCAN:</span>
                    <span className="text-white">2 分钟前</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-500">UPTIME:</span>
                    <span className="text-green-400 animate-pulse">00:03:35</span>
                  </div>
                </div>

                <div className="mt-6 border-t border-dashed border-gray-700 pt-4 text-center">
                   <p className="text-xs text-gray-600 tracking-[0.2em]">END OF TAPE</p>
                </div>
              </div>
            </div>

          </div>
        </div>
      </div>
      
      {/* Global Styles for Scrollbar */}
      <style jsx global>{`
        .custom-scrollbar::-webkit-scrollbar {
          width: 12px;
        }
        .custom-scrollbar::-webkit-scrollbar-track {
          background: #e8e4d9;
          border-left: 2px solid #1f2937;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb {
          background: #1f2937;
          border: 2px solid #e8e4d9;
        }
        .custom-scrollbar::-webkit-scrollbar-thumb:hover {
          background: #ea580c;
        }
      `}</style>
    </div>
  );
}