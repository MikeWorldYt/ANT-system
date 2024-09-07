import { AfterViewInit, Component, ElementRef, OnInit, QueryList, ViewChildren } from '@angular/core';
import { CommonModule } from '@angular/common';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { Language } from '../../../services/language.types';
import { PageService } from '../../../services/navPage.service';

// ▲ CONTENT ▲
import { content } from '../../content/content';

// Testing
import { HeaderT02Component } from '../header-t02/header-t02.component';

@Component({
  selector: 'doc-t02-mod-02-support-folders',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule
  ],
  templateUrl: './mod-02-support-folders.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod02_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: ArticleService,
    private pageService: PageService,
    private languageService: LanguageService,
  ) { }
  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'ES';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  // ▲ service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  //  ████ OnInit ████
  ngOnInit(): void {
    // For inner content
    this.write = content[this.currentLanguage].title_02.page_02;
    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe((language: string) => {
        this.write = content[language].title_02.page_02;
        //this.currentLanguage = language as Language;
    });
    // Set currentPage
    this.pageService.setCurrentPage('page_02');
  }
  
  // ████ Hash Sections
  hovered = false;
  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });

    // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(id => {
      this.currentArticle = id;
    })
  }

}