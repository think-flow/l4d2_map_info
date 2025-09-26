use anyhow::{Result as AnyResult, anyhow, bail};
use crossterm::{cursor::*, event::*, execute, style::*, terminal::*};
use map_info::vpk::*;
use regex::bytes::Regex;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs,
    io::stdout,
    path::PathBuf,
};
use strsim::{jaro_winkler, levenshtein};

/// 保留行数， 1行提示，1行输入，1行页码输出
const RESERVED_ROWS: u16 = 3;

struct Mode1State {
    /// 用于缓存输入
    input_buffer: VecDeque<char>,
    /// 用于存储输出区域的光标起始位置
    input_cursor_pos: (u16, u16),
    /// 当前选中项的索引（-1为未选中）
    selected_index: i32,
    /// 匹配到的文件列表 (None和Vec.len =0 时为空)
    matched_file_list: Option<Vec<String>>,
    /// 文件名到文件路径的映射
    file_map: HashMap<String, PathBuf>,
    /// 列表偏移量
    page_offset: usize,
    /// 列表大小
    page_size: usize,
    terminal_size: (u16, u16),
}

struct Mode2State {
    /// 用于缓存输入
    input_buffer: VecDeque<char>,
    /// 用于存储输出区域的光标起始位置
    input_cursor_pos: (u16, u16),
    /// 当前选中项的索引（-1为未选中）
    selected_index: i32,
    /// 1. code模式 2. file模式
    mode: u8,
    /// 匹配到的地图代码列表 (None和Vec.len =0 时为空)
    matched_code_list: Option<Vec<String>>,
    /// 匹配到的文件列表 (None和Vec.len =0 时为空)
    matched_file_list: Option<Vec<String>>,
    /// 文件名到文件内容的映射
    file_map: HashMap<String, String>,
    /// 地图代码到文件集合的映射
    code_map: HashMap<String, HashSet<String>>,
    /// 列表偏移量
    page_offset: usize,
    /// 列表大小
    page_size: usize,
    terminal_size: (u16, u16),
}

impl Mode1State {
    /// 处理键盘事件
    fn handle_key_event(&mut self, event: KeyEvent) -> AnyResult<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }
        let is_render = match event.code {
            KeyCode::Char(c) => {
                // 可见字符需要回显控制台
                self.input_buffer.push_back(c);
                self.selected_index = -1;
                self.page_offset = 0;
                true
            }
            KeyCode::Backspace => {
                // 删除一个字符
                if self.input_buffer.pop_back().is_some() {
                    self.selected_index = -1;
                    self.page_offset = 0;
                    true
                } else {
                    false
                }
            }
            KeyCode::Enter => {
                let selected = self.selected_index;
                let path: String;
                if selected >= 0 {
                    path = self.matched_file_list.as_ref().unwrap()[selected as usize].clone();
                } else {
                    path = String::from_iter(self.input_buffer.iter());
                    if path.is_empty() {
                        bail!("路径不能为空")
                    }
                }
                // 清空输入缓冲区和光标位置
                // self.input_buffer.clear();

                /*
                    输出文件内容
                    由于备用缓冲区，无法滚动，所以只能转为主缓冲区输出内容，并退出程序了
                */
                let path = self
                    .file_map
                    .get(&path)
                    .ok_or(anyhow!("路径{}不存在", path))?;

                let vpk_info = VPKInfo::new(path)?;
                let mission_info = vpk_info.get_mission()?;

                print_output(&mission_info);
            }
            // 下方向键 选择列表
            KeyCode::Down => {
                let index = &mut self.selected_index;
                let file_count = self.matched_file_list.as_ref().map_or(0, |v| v.len()) as i32;
                if *index >= file_count - 1 {
                    *index = file_count - 1;
                    false
                } else {
                    *index += 1;
                    // 计算page_offset
                    if *index == (self.page_offset + self.page_size) as i32 {
                        self.page_offset += 1;
                    }
                    true
                }
            }
            // 上方向键 选择列表
            KeyCode::Up => {
                let index = &mut self.selected_index;
                if *index < 0 {
                    *index = -1;
                    false
                } else {
                    *index -= 1;
                    // 计算page_offset
                    let page_offset = self.page_offset as i32;
                    if page_offset > 0 && *index + 1 == page_offset {
                        self.page_offset -= 1;
                    }
                    true
                }
            }
            // 自动补全
            KeyCode::Tab => {
                if self.matched_file_list.as_ref().map_or(0, |v| v.len()) > 0 {
                    let mut index = 0;
                    if self.selected_index >= 0 {
                        index = self.selected_index;
                    }
                    let first_file = &self.matched_file_list.as_ref().unwrap()[index as usize];
                    self.selected_index = -1;
                    self.page_offset = 0;
                    self.input_buffer.clear();
                    self.input_buffer.extend(first_file.chars());
                    true
                } else {
                    false
                }
            }
            _ => false,
        };

        if is_render {
            self.match_file_name();
            // 渲染input
            self.render_input()?;
            // 渲染list
            self.render_list()?;
        }

        Ok(())
    }

    /// input为空，则返回None，如果没有匹配项则vec len为0
    fn match_file_name(&mut self) {
        let input = String::from_iter(self.input_buffer.iter());
        if input.is_empty() {
            self.matched_file_list = None;
        } else {
            // 获取阈值
            let threshold = dyn_threadhold(&input);

            let mut matched_files: Vec<(String, f64)> = self
                .file_map
                .keys()
                .flat_map(|file_name| {
                    let score = score_keyword(&input, file_name);
                    if score >= threshold {
                        Some((file_name.to_owned(), score))
                    } else {
                        None
                    }
                })
                .collect();
            matched_files.sort_by(|a, b| b.1.total_cmp(&a.1));

            self.matched_file_list = Some(matched_files.into_iter().map(|v| v.0).collect());
        }
    }

    fn render_input(&self) -> AnyResult<()> {
        let input = String::from_iter(self.input_buffer.iter());
        let (col, row) = self.input_cursor_pos;
        execute!(stdout(), MoveTo(col, row), Clear(ClearType::UntilNewLine))?;

        if input.len() > 0 {
            execute!(stdout(), Print(input))?;
        }

        Ok(())
    }

    fn render_list(&self) -> AnyResult<()> {
        // 清空，显示区域
        execute!(stdout(), SavePosition, MoveToNextLine(1))?;

        match &self.matched_file_list {
            None => {
                execute!(stdout(), Clear(ClearType::FromCursorDown), RestorePosition)?;
                Ok(())
            }
            Some(file_list) => {
                if file_list.is_empty() {
                    execute!(
                        stdout(),
                        Clear(ClearType::FromCursorDown),
                        Print("无匹配项"),
                        RestorePosition
                    )?;
                    return Ok(());
                } else {
                    // 支持列表滚动
                    let page_offset = self.page_offset;
                    let page_size = self.page_size;
                    let mut count = 0;
                    for (i, file) in file_list
                        .iter()
                        .skip(page_offset)
                        .take(page_size)
                        .enumerate()
                    {
                        if (i + page_offset) as i32 == self.selected_index {
                            execute!(
                                stdout(),
                                Clear(ClearType::CurrentLine),
                                SetAttribute(Attribute::Underlined),
                                Print(format!("{}. {}\n", i + 1 + page_offset, file)),
                                SetAttribute(Attribute::Reset),
                            )?;
                        } else {
                            execute!(
                                stdout(),
                                Clear(ClearType::CurrentLine),
                                Print(format!("{}. {}\n", i + 1 + page_offset, file))
                            )?;
                        }
                        count += 1;
                    }
                    // 显示页码
                    execute!(
                        stdout(),
                        Clear(ClearType::CurrentLine),
                        Print(format!("[{}/{}]", self.selected_index + 1, file_list.len()))
                    )?;

                    // 如果打印的行数，小于控制台的行数，则填充空白行
                    if count < self.terminal_size.1 {
                        let empty_str = " ".repeat(self.terminal_size.0 as usize);
                        for _ in 0..(self.terminal_size.1 - count - RESERVED_ROWS) {
                            execute!(stdout(), Print(format!("\n{}", empty_str)))?;
                        }
                    }

                    execute!(stdout(), RestorePosition)?;
                    Ok(())
                }
            }
        }
    }
}

impl Mode2State {
    fn handle_key_event(&mut self, event: KeyEvent) -> AnyResult<()> {
        if event.kind != KeyEventKind::Press {
            return Ok(());
        }
        let is_render = match event.code {
            KeyCode::Char(c) => {
                if self.mode == 2 {
                    false
                } else {
                    // 可见字符需要回显控制台
                    self.input_buffer.push_back(c);
                    self.selected_index = -1;
                    self.page_offset = 0;
                    true
                }
            }
            KeyCode::Backspace => {
                if self.mode == 2 {
                    false
                } else {
                    // 删除一个字符
                    if self.input_buffer.pop_back().is_some() {
                        self.selected_index = -1;
                        self.page_offset = 0;
                        true
                    } else {
                        false
                    }
                }
            }
            KeyCode::Enter => {
                if self.mode == 1 {
                    // code模式
                    let selected = self.selected_index;
                    let code: String;
                    if selected >= 0 {
                        code = self.matched_code_list.as_ref().unwrap()[selected as usize].clone();
                    } else {
                        code = String::from_iter(self.input_buffer.iter());
                        if code.is_empty() {
                            bail!("建图代码不能为空")
                        }
                    }

                    let file_list = self
                        .code_map
                        .get(&code)
                        .ok_or(anyhow!("建图代码{}不存在", code))?;

                    if file_list.len() == 1 {
                        /*
                            输出文件内容
                            由于备用缓冲区，无法滚动，所以只能转为主缓冲区输出内容，并退出程序了
                        */
                        let file = file_list.iter().next().unwrap();
                        let mission_info = self
                            .file_map
                            .get(file)
                            .ok_or(anyhow!("路径{}不存在", file))?;

                        print_output(mission_info);
                    } else {
                        //进入模式2 选择文件模式
                        self.mode = 2;
                        self.matched_file_list =
                            Some(file_list.iter().map(|v| v.to_owned()).collect());
                        self.selected_index = 0;
                        self.page_offset = 0;
                        execute!(
                            stdout(),
                            MoveTo(0, 0),
                            Clear(ClearType::All),
                            Hide,
                            Print("请选择要查询的文件名：")
                        )?;
                        true
                    }
                } else {
                    // file模式
                    let selected = self.selected_index;
                    let file = &self.matched_file_list.as_ref().unwrap()[selected as usize];
                    let mission_info = self
                        .file_map
                        .get(file)
                        .ok_or(anyhow!("路径{}不存在", file))?;

                    /*
                        输出文件内容
                        由于备用缓冲区，无法滚动，所以只能转为主缓冲区输出内容，并退出程序了
                    */
                    print_output(mission_info);
                }
            }
            // 下方向键 选择列表
            KeyCode::Down => {
                if self.mode == 1 {
                    // 代码选择模式
                    let index = &mut self.selected_index;
                    let count = self.matched_code_list.as_ref().map_or(0, |v| v.len()) as i32;
                    if *index >= count - 1 {
                        *index = count - 1;
                        false
                    } else {
                        *index += 1;
                        // 计算page_offset
                        if *index == (self.page_offset + self.page_size) as i32 {
                            self.page_offset += 1;
                        }
                        true
                    }
                } else {
                    // 文件选择模式
                    let index = &mut self.selected_index;
                    let count = self.matched_file_list.as_ref().map_or(0, |v| v.len()) as i32;
                    if *index >= count - 1 {
                        *index = count - 1;
                        false
                    } else {
                        *index += 1;
                        // 计算page_offset
                        if *index == (self.page_offset + self.page_size) as i32 {
                            self.page_offset += 1;
                        }
                        true
                    }
                }
            }
            // 上方向键 选择列表
            KeyCode::Up => {
                if self.mode == 1 {
                    // 代码选择模式
                    let index = &mut self.selected_index;
                    if *index < 0 {
                        *index = -1;
                        false
                    } else {
                        *index -= 1;
                        // 计算page_offset
                        let page_offset = self.page_offset as i32;
                        if page_offset > 0 && *index + 1 == page_offset {
                            self.page_offset -= 1;
                        }
                        true
                    }
                } else {
                    // 文件选择模式
                    let index = &mut self.selected_index;
                    if *index <= 0 {
                        *index = 0;
                        false
                    } else {
                        *index -= 1;
                        // 计算page_offset
                        let page_offset = self.page_offset as i32;
                        if page_offset > 0 && *index + 1 == page_offset {
                            self.page_offset -= 1;
                        }
                        true
                    }
                }
            }
            // 自动补全
            KeyCode::Tab => {
                if self.mode == 1 {
                    // 代码选择模式 (不做任何事)
                    if self.matched_code_list.as_ref().map_or(0, |v| v.len()) > 0 {
                        let mut index = 0;
                        if self.selected_index >= 0 {
                            index = self.selected_index;
                        }
                        let first_file = &self.matched_code_list.as_ref().unwrap()[index as usize];
                        self.selected_index = -1;
                        self.page_offset = 0;
                        self.input_buffer.clear();
                        self.input_buffer.extend(first_file.chars());
                        true
                    } else {
                        false
                    }
                } else {
                    // 文件选择模式
                    false
                }
            }
            _ => false,
        };

        if is_render {
            if self.mode == 1 {
                self.match_map_code();
                // 渲染input
                self.render_input()?;
            }
            // 渲染list
            self.render_list()?;
        }

        Ok(())
    }

    fn render_input(&self) -> AnyResult<()> {
        let input = String::from_iter(self.input_buffer.iter());
        let (col, row) = self.input_cursor_pos;
        execute!(stdout(), MoveTo(col, row), Clear(ClearType::UntilNewLine))?;

        if input.len() > 0 {
            execute!(stdout(), Print(input))?;
        }

        Ok(())
    }

    fn match_map_code(&mut self) {
        let input = String::from_iter(self.input_buffer.iter());
        if input.is_empty() {
            self.matched_code_list = None;
        } else {
            // 获取阈值
            let threshold = dyn_threadhold(&input);

            let mut matched_codes: Vec<(String, f64)> = self
                .code_map
                .keys()
                .flat_map(|code| {
                    let score = score_keyword(&input, code);
                    if score >= threshold {
                        Some((code.to_owned(), score))
                    } else {
                        None
                    }
                })
                .collect();
            matched_codes.sort_by(|a, b| b.1.total_cmp(&a.1));

            self.matched_code_list = Some(matched_codes.into_iter().map(|v| v.0).collect());
        }
    }

    fn render_list(&self) -> AnyResult<()> {
        // 清空，显示区域
        execute!(stdout(), SavePosition, MoveToNextLine(1))?;

        if self.mode == 1 {
            // 代码选择模式
            match &self.matched_code_list {
                None => {
                    execute!(stdout(), Clear(ClearType::FromCursorDown), RestorePosition)?;
                    Ok(())
                }
                Some(code_list) => {
                    if code_list.is_empty() {
                        execute!(
                            stdout(),
                            Clear(ClearType::FromCursorDown),
                            Print("无匹配项"),
                            RestorePosition
                        )?;
                        return Ok(());
                    } else {
                        let page_offset = self.page_offset;
                        let page_size = self.page_size;
                        let mut count = 0;
                        for (i, code) in code_list
                            .iter()
                            .skip(page_offset)
                            .take(page_size)
                            .enumerate()
                        {
                            let file_list = self.code_map.get(code).unwrap();
                            // 显示文件名
                            if (i + page_offset) as i32 == self.selected_index {
                                execute!(
                                    stdout(),
                                    Clear(ClearType::CurrentLine),
                                    SetAttribute(Attribute::Underlined),
                                    Print(format!(
                                        "{}. {} => {}\n",
                                        i + 1 + page_offset,
                                        code,
                                        Self::format_hash_set(file_list)
                                    )),
                                    SetAttribute(Attribute::Reset),
                                )?;
                            } else {
                                execute!(
                                    stdout(),
                                    Clear(ClearType::CurrentLine),
                                    Print(format!(
                                        "{}. {} => {}\n",
                                        i + 1 + page_offset,
                                        code,
                                        Self::format_hash_set(file_list)
                                    ))
                                )?;
                            }
                            count += 1;
                        }
                        // 显示页码
                        execute!(
                            stdout(),
                            Clear(ClearType::CurrentLine),
                            Print(format!("[{}/{}]", self.selected_index + 1, code_list.len()))
                        )?;

                        // 如果打印的行数，小于控制台的行数，则填充空白行
                        if count < self.terminal_size.1 {
                            let empty_str = " ".repeat(self.terminal_size.0 as usize);
                            for _ in 0..(self.terminal_size.1 - count - RESERVED_ROWS) {
                                execute!(stdout(), Print(format!("\n{}", empty_str)))?;
                            }
                        }

                        execute!(stdout(), RestorePosition)?;
                        Ok(())
                    }
                }
            }
        } else {
            // 文件选择模式
            match &self.matched_file_list {
                None => {
                    execute!(stdout(), RestorePosition)?;
                    Ok(())
                }
                Some(file_list) => {
                    let page_offset = self.page_offset;
                    let page_size = self.page_size;
                    for (i, code) in file_list
                        .iter()
                        .skip(page_offset)
                        .take(page_size)
                        .enumerate()
                    {
                        // 显示文件名
                        if (i + page_offset) as i32 == self.selected_index {
                            execute!(
                                stdout(),
                                SetAttribute(Attribute::Underlined),
                                Print(format!("{}. {}\n", i + 1 + page_offset, code)),
                                SetAttribute(Attribute::Reset),
                            )?;
                        } else {
                            execute!(
                                stdout(),
                                Print(format!("{}. {}\n", i + 1 + page_offset, code))
                            )?;
                        }
                    }
                    execute!(
                        stdout(),
                        Print(format!("[{}/{}]", self.selected_index + 1, file_list.len())),
                        RestorePosition
                    )?;
                    Ok(())
                }
            }
        }
    }

    fn format_hash_set(set: &HashSet<String>) -> String {
        // ""
        if set.is_empty() {
            return "".to_string();
        }
        // "a.vpk"
        if set.len() == 1 {
            return format!("\"{}\"", set.iter().next().unwrap());
        }

        // "a.vpk", "b.vpk"
        let temp = set
            .iter()
            .map(|s| format!("\"{}\"", s))
            .collect::<Vec<_>>()
            .join(", ");

        // ["a.vpk", "b.vpk"]
        let string = format!("[{}]", temp);
        return string;
    }
}

fn main() -> AnyResult<()> {
    init_screen().map_err(|e| {
        close_screen();
        e
    })?;

    let mode = chose_search_mode().map_err(|e| {
        close_screen();
        e
    })?;
    // let mode = 2;

    match mode {
        1 => {
            return mode_1().map_err(|e| {
                close_screen();
                e
            });
        }
        2 => {
            return mode_2().map_err(|e| {
                close_screen();
                e
            });
        }
        3 => {
            return mode_3().map_err(|e| {
                close_screen();
                e
            });
        }
        _ => bail!("无效的模式"),
    }
}

fn init_screen() -> AnyResult<()> {
    enable_raw_mode()?;
    execute!(
        stdout(),
        DisableLineWrap, // 禁用自动换行功能
        EnterAlternateScreen,
        MoveTo(0, 0)
    )?;
    Ok(())
}

fn close_screen() {
    _ = execute!(stdout(), EnableLineWrap, LeaveAlternateScreen, Show);
    _ = disable_raw_mode();
}

fn chose_search_mode() -> AnyResult<u32> {
    let hint = r#"请选择查询模式：
1. 根据文件查询地图信息
2. 根据建图代码查询地图信息
3. 查找重复地图代码的文件
> "#;
    execute!(stdout(), Print(hint))?;
    let result: u32 = loop {
        match read()? {
            Event::Key(event) => {
                // 按下 Ctrl+C 退出程序
                if event.code == KeyCode::Char('c') && event.modifiers == KeyModifiers::CONTROL {
                    close_screen();
                    std::process::exit(0);
                }
                if event.kind == KeyEventKind::Press && event.modifiers == KeyModifiers::NONE {
                    match event.code {
                        KeyCode::Char('1') => {
                            execute!(stdout(), Print("1"))?;
                            break 1;
                        }
                        KeyCode::Char('2') => {
                            execute!(stdout(), Print("2"))?;
                            break 2;
                        }
                        KeyCode::Char('3') => {
                            execute!(stdout(), Print("3"))?;
                            break 3;
                        }
                        _ => continue,
                    }
                }
            }
            _ => continue,
        }
    };
    // clear all screen
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(result)
}

fn mode_1() -> AnyResult<()> {
    let mut current_dir = std::env::current_dir()?;
    if cfg!(debug_assertions) {
        current_dir = PathBuf::from(
            r"C:\Program Files (x86)\Steam\steamapps\common\Left 4 Dead 2\left4dead2\addons",
        );
    }

    // 获取目录第一层下的所有文件
    let files = fs::read_dir(current_dir)?
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    // eprintln!("警告: 忽略无法访问的目录项: {}", e);
                    return None;
                }
            };

            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            //path 扩展名不为 .vpk则忽略
            if path.extension().unwrap_or_default() != "vpk" {
                return None;
            }

            Some((path.file_name().unwrap().to_str().unwrap().to_owned(), path))
        })
        .collect::<HashMap<_, _>>();

    // 计算page_size
    let terminal_size = size()?;
    let page_size = (terminal_size.1 - RESERVED_ROWS) as usize;
    if page_size < 1 {
        bail!("屏幕高度过小")
    }

    execute!(stdout(), Print("请输入要查询的文件名：\n> "))?;

    // 构建存储状态的结构体
    let mut state = Mode1State {
        input_buffer: VecDeque::new(),
        input_cursor_pos: position()?,
        selected_index: -1,
        matched_file_list: None,
        file_map: files,
        page_offset: 0,
        page_size: page_size,
        terminal_size: terminal_size,
    };

    loop {
        match read()? {
            Event::Key(event) => {
                // 按下 Ctrl+C 退出程序
                if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('c') {
                    close_screen();
                    std::process::exit(0);
                }
                state.handle_key_event(event)?;
            }
            _ => continue,
        }
    }
}

fn mode_2() -> AnyResult<()> {
    let mut current_dir = std::env::current_dir()?;
    if cfg!(debug_assertions) {
        current_dir = PathBuf::from(
            r"C:\Program Files (x86)\Steam\steamapps\common\Left 4 Dead 2\left4dead2\addons",
        );
    }

    // 获取目录第一层下的所有文件
    let files = fs::read_dir(current_dir)?
        .filter_map(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    // eprintln!("警告: 忽略无法访问的目录项: {}", e);
                    return None;
                }
            };

            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            //path 扩展名不为 .vpk则忽略
            if path.extension().unwrap_or_default() != "vpk" {
                return None;
            }

            match VPKInfo::new(&path) {
                Ok(vpk_info) => match vpk_info.get_mission() {
                    Ok(mission) => Some((
                        path.file_name().unwrap().to_str().unwrap().to_owned(),
                        mission,
                    )),
                    Err(_) => None,
                },
                Err(_) => None,
            }
        })
        .collect::<HashMap<_, _>>();

    let mut map_code_map: HashMap<String, HashSet<String>> = HashMap::new();
    let regex = Regex::new(r#"(?i)"map"\s*"([^"]+)""#).map_err(|e| anyhow!(e))?;
    for (file_name, content) in files.iter() {
        let map_list = extract_coop_maps(content, &regex);
        for map_code in map_list {
            let entry = map_code_map
                .entry(map_code.to_owned())
                .or_insert(HashSet::new());
            entry.insert(file_name.to_owned());
        }
    }

    // 计算page_size
    let terminal_size = size()?;
    let page_size = (terminal_size.1 - RESERVED_ROWS) as usize;
    if page_size < 1 {
        bail!("屏幕高度过小")
    }

    execute!(stdout(), Print("请输入要查询的建图代码：\n> "))?;

    // 构建存储状态的结构体
    let mut state = Mode2State {
        input_buffer: VecDeque::new(),
        input_cursor_pos: position()?,
        selected_index: -1,
        file_map: files,
        code_map: map_code_map,
        matched_code_list: None,
        matched_file_list: None,
        mode: 1,
        page_offset: 0,
        page_size: page_size,
        terminal_size,
    };

    loop {
        match read()? {
            Event::Key(event) => {
                // 按下 Ctrl+C 退出程序
                if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('c') {
                    close_screen();
                    std::process::exit(0);
                }
                state.handle_key_event(event)?;
            }
            _ => continue,
        }
    }
}

fn mode_3() -> AnyResult<()> {
    let mut current_dir = std::env::current_dir()?;
    if cfg!(debug_assertions) {
        current_dir = PathBuf::from(
            r"C:\Program Files (x86)\Steam\steamapps\common\Left 4 Dead 2\left4dead2\addons",
        );
    }

    let regex = Regex::new(r#"(?i)"map"\s*"([^"]+)""#).map_err(|e| anyhow!(e))?;
    let mut map_code_map: HashMap<String, HashSet<String>> = HashMap::new();

    let instant = std::time::Instant::now();
    for entry in fs::read_dir(current_dir)? {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                // eprintln!("警告: 忽略无法访问的目录项: {}", e);
                continue;
            }
        };

        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        execute!(
            stdout(),
            Clear(ClearType::CurrentLine),
            MoveToColumn(0),
            Print(format!("正在分析: {}", path.display())),
        )
        .unwrap();

        //path 扩展名不为 .vpk则忽略
        if path.extension().unwrap_or_default() != "vpk" {
            continue;
        }
        if let Ok(vpk_info) = VPKInfo::new(&path) {
            if let Ok(mission) = vpk_info.get_mission() {
                let map_list = extract_coop_maps(&mission, &regex);
                for map_code in map_list {
                    let entry = map_code_map
                        .entry(map_code.to_owned())
                        .or_insert(HashSet::new());
                    entry.insert(
                        path.file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
            }
        }
    }

    map_code_map.retain(|_, v| v.len() > 1);

    let mut msg = String::new();
    let set = map_code_map
        .values()
        .map(|v| {
            let mut tmp = v.iter().map(|v| v.to_owned()).collect::<Vec<_>>();
            tmp.sort();
            tmp.join(",")
        })
        .collect::<HashSet<_>>();

    let past_time = instant.elapsed();

    if set.is_empty() {
        msg.push_str("没有找到重复的地图文件");
    } else {
        msg.push_str("找到以下地图代码相同的文件组：\n");
        for (i, str) in set.iter().enumerate() {
            msg.push_str(format!("\n组 {}:\n", i + 1).as_str());
            let file_list: Vec<&str> = str.split(",").collect();
            for filepath in file_list {
                msg.push_str(format!("  - {}\n", filepath).as_str());
            }
        }
    }
    msg.push_str(format!("\n耗时: {:.2}s", past_time.as_secs_f32()).as_str());
    print_output(msg.as_str());
}

/// 提取 coop 下所有 Map 名称
fn extract_coop_maps<'a>(text: &'a str, regex: &Regex) -> Vec<&'a str> {
    let mut results = Vec::new();
    for cap in regex.captures_iter(text.as_bytes()) {
        if let Ok(str) = std::str::from_utf8(cap.get(1).unwrap().as_bytes()) {
            results.push(str);
        }
    }
    results
}

fn print_output(output: &str) -> ! {
    close_screen();
    println!("{}", output);
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).unwrap();
    std::process::exit(0);
}

/// 给定关键字和文本，返回一个相似度分数（0.0 ~ 1.0）
fn score_keyword(keyword: &str, txt: &str) -> f64 {
    let keyword_lower = keyword.to_lowercase();
    let txt_lower = txt.to_lowercase();

    // 子串命中直接加权（最高优先）
    if txt_lower.contains(&keyword_lower) {
        return 1.0;
    }

    // 计算 jaro-winkler 相似度
    let jw_score = jaro_winkler(&txt_lower, &keyword_lower);

    // 计算 Levenshtein 距离，转成相似度
    let max_len = txt_lower.len().max(keyword_lower.len());
    let lev_dist = levenshtein(&txt_lower, &keyword_lower);
    let lev_score = 1.0 - (lev_dist as f64 / max_len as f64);

    // 这里我们取平均分（可以调整权重）
    (jw_score + lev_score) / 2.0
}

/// 根据输出字符串，获得阈值
fn dyn_threadhold(keyword: &str) -> f64 {
    let input_len = keyword.chars().count();
    let mut threshold = 0.5;
    if input_len <= 4 {
        threshold = 0.1 * input_len as f64;
    } else if input_len >= 8 {
        threshold = 0.8;
    }
    threshold
}
